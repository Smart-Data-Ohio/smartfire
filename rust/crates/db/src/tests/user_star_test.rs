//! test/models/user_star_test.rb plus real independent SQLite writers.
use super::*;
use crate::{User, UserStar};

#[test]
fn ws12_star_model_enforces_uniqueness_and_self_rejection() {
    let t = TestDb::with_clock(TestClock::frozen_at(crate::Timestamp::from_second(1772467200)),4);
    let (viewer, target) = (id("david"), id("kevin"));
    let star = t.write(move |tx| UserStar::create(tx, viewer, target));
    assert_eq!(star.created_at, t.now());
    let error = t.try_write(move |tx| UserStar::create(tx, viewer, target)).unwrap_err();
    assert!(matches!(error, crate::Error::RecordInvalid(ref e) if e.on("starred_user_id")==["has already been taken"]));
    let error = t.try_write(move |tx| {
        tx.conn().execute("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![viewer,target,tx.now(),tx.now()])?;
        Ok(())
    }).unwrap_err();
    assert!(error.is_record_not_unique());
    for viewer in [viewer, id("bender")] {
        let error = t.try_write(move |tx| UserStar::create(tx, viewer, viewer)).unwrap_err();
        assert!(matches!(error, crate::Error::RecordInvalid(ref e) if e.on("starred_user")==["must be someone else"]));
    }
}

#[test]
fn ws12_star_model_requires_both_associations() {
    let t = TestDb::new();
    for (viewer, target, field) in [(-1,id("kevin"),"user"),(id("david"),-1,"starred_user")] {
        let error = t.try_write(move |tx| UserStar::create(tx, viewer, target)).unwrap_err();
        assert!(matches!(error, crate::Error::RecordInvalid(ref e) if e.on(field)==["must exist"]));
    }
}

#[test]
fn ws12_star_readers_and_deletes_are_viewer_scoped() {
    let t = TestDb::new();
    let (david,jason,kevin) = (id("david"),id("jason"),id("kevin"));
    t.write(move |tx| {
        UserStar::create(tx,david,kevin)?;
        UserStar::create(tx,jason,kevin)?;
        UserStar::create(tx,david,id("bender"))?;
        Ok(())
    });
    t.read(|conn| {
        let user = User::find(conn,david)?;
        assert!(user.starred(conn,kevin)?);
        assert!(!user.starred(conn,jason)?);
        assert_eq!(user.starred_ids_among(conn,&[kevin,jason,kevin])?,std::collections::HashSet::from([kevin]));
        assert!(user.starred_ids_among(conn,&[])?.is_empty());
        Ok(())
    });
    assert_eq!(t.write(move |tx| UserStar::remove(tx,david,kevin)),1);
    assert_eq!(t.write(move |tx| UserStar::remove(tx,david,kevin)),0);
    assert!(t.read(|conn| UserStar::find(conn,jason,kevin)).is_some());
    assert!(t.read(|conn| UserStar::find(conn,david,id("bender"))).is_some());
    assert!(t.events().is_empty(),"stars are private and never broadcast or notify");
}

#[test]
fn ws12_star_deactivation_retains_rows_and_hard_removal_deletes_both_sides() {
    let t = TestDb::new();
    // New users avoid unrelated huddle/calendar fixture dependencies in User#destroy.
    let (a,b,c) = t.write(|tx| {
        let a = User::create(tx,crate::NewUser {name:"Starrer".into(),..Default::default()})?;
        let b = User::create(tx,crate::NewUser {name:"Target".into(),..Default::default()})?;
        let c = User::create(tx,crate::NewUser {name:"Other starrer".into(),..Default::default()})?;
        UserStar::create(tx,a.id,b.id)?;
        UserStar::create(tx,c.id,b.id)?;
        Ok((a,b,c))
    });
    let b_id = b.id;
    let a_id = a.id;
    let c_id = c.id;
    t.write(move |tx| {let mut b=b; b.deactivate(tx)});
    assert!(t.read(|conn| UserStar::find(conn,a_id,b_id)).is_some());
    t.write(move |tx| a.destroy(tx));
    assert!(t.read(|conn| UserStar::find(conn,a_id,b_id)).is_none());
    assert!(t.read(|conn| UserStar::find(conn,c_id,b_id)).is_some());
    t.write(move |tx| User::find(tx.conn(),b_id)?.destroy(tx));
    assert!(t.read(|conn| UserStar::find(conn,c_id,b_id)).is_none());
}

#[test]
fn ws12_star_find_or_create_serializes_independent_writers_without_touching() {
    let t = TestDb::new();
    let one = t.another_process();
    let two = t.another_process();
    let (viewer,target) = (id("david"),id("kevin"));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writers = [one,two].map(|db| {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            db.write_blocking(move |tx| UserStar::find_or_create(tx,viewer,target)).unwrap()
        })
    });
    let [one,two] = writers.map(|writer| writer.join().unwrap());
    assert_eq!(one,two);
    t.clock.travel_to(t.now().since(jiff::SignedDuration::from_hours(1)));
    assert_eq!(t.write(move |tx| UserStar::find_or_create(tx,viewer,target)),one);
}
