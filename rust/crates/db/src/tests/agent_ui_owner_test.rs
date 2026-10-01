use super::*;
use crate::models::user::icon;
use crate::{Agent, AgentChanges, Error, NewUser, Timestamp, User, UserChanges};
use rusqlite::params;
use serde_json::{Value, json};
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_ui_owner_inputs_contract.json"
    ))
    .unwrap()
}
#[test]
fn ws11_ui_owner_icons_match_rails_create_update_clear_and_avatar_stamp() {
    let t = TestDb::new();
    for row in gold()["icons"].as_array().unwrap() {
        if !row["permitted"].as_bool().unwrap() {
            continue;
        }
        let name = icon::normalize_input(&row["input"]);
        assert_eq!(json!(name), row["value"]);
        let expected = row["errors"].clone();
        let before = t.read(User::count);
        let result = t.try_write(move |tx| {
            User::create_bot_with_attributes(
                tx,
                NewUser {
                    name: "Icon contract".into(),
                    icon_name: name,
                    ..Default::default()
                },
                None,
            )
        });
        if expected.as_array().unwrap().is_empty() {
            let bot = result.unwrap();
            assert_eq!(json!(bot.icon_name), row["value"]);
        } else {
            let Error::RecordInvalid(e) = result.unwrap_err() else {
                panic!("expected icon validation");
            };
            assert_eq!(json!(e.on("icon_name")), expected);
            assert_eq!(t.read(User::count), before);
        }
        let name = icon::normalize_input(&row["input"]);
        let result = t.try_write(move |tx| {
            User::find(tx.conn(), id("bender"))?.update(
                tx,
                UserChanges {
                    icon_name: Some(name),
                    ..Default::default()
                },
            )
        });
        if expected.as_array().unwrap().is_empty() {
            result.unwrap();
        } else {
            let Error::RecordInvalid(e) = result.unwrap_err() else {
                panic!("expected update validation");
            };
            assert_eq!(json!(e.on("icon_name")), expected);
        }
    }
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET icon_name=NULL,updated_at=? WHERE id=?",
            params![
                tx.now().ago(jiff::SignedDuration::from_secs(2 * 86400)),
                id("bender")
            ],
        )?;
        Ok(())
    });
    let before = t.read(|c| User::find(c, id("bender")));
    let bot = t.write(|tx| {
        let mut bot = User::find(tx.conn(), id("bender"))?;
        bot.set_icon_name(tx, Some(":OpenAI:"))?;
        Ok(bot)
    });
    assert_eq!(
        json!({"value":bot.icon_name,"timestamp_changed":bot.updated_at!=before.updated_at}),
        gold()["icon_write"]
    );
    t.travel(2);
    let unchanged = t.write(|tx| {
        let mut bot = User::find(tx.conn(), id("bender"))?;
        bot.set_icon_name(tx, Some(" ::: OPENAI ::: "))?;
        Ok(bot)
    });
    assert_eq!(
        json!({"value":unchanged.icon_name,"timestamp_changed":unchanged.updated_at!=bot.updated_at}),
        gold()["icon_noop"]
    );
    t.write(|tx| {
        let mut bot = User::find(tx.conn(), id("bender"))?;
        bot.set_icon_name(tx, Some(""))?;
        assert_eq!(json!(bot.icon_name), gold()["icon_clear"]);
        Ok(())
    });
    // Validation reads custom icons live, including removal after a prior successful save.
    t.write(|tx| {tx.conn().execute("INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES ('ws11_custom','Custom',?, ?,?)",params![id("david"),tx.now(),tx.now()])?;let mut bot=User::find(tx.conn(),id("bender"))?;bot.set_icon_name(tx,Some(":ws11_custom:"))?;tx.conn().execute("DELETE FROM workspace_icons WHERE name='ws11_custom'",[])?;bot.set_icon_name(tx,Some("ws11_custom"))?;bot.set_icon_name(tx,None)?;assert!(bot.set_icon_name(tx,Some("ws11_custom")).is_err());Ok(())});
}
#[test]
fn ws11_ui_owner_raw_caps_preserve_rails_cast_errors_and_form_values() {
    let t = TestDb::new();
    for row in gold()["caps"].as_array().unwrap() {
        if !row["permitted"].as_bool().unwrap() {
            continue;
        }
        let field = row["field"].as_str().unwrap();
        let mut changes = AgentChanges::default();
        match field {
            "daily_message_cap" => {
                changes.daily_message_cap_before_type_cast = Some(row["input"].clone())
            }
            "daily_board_post_cap" => {
                changes.daily_board_post_cap_before_type_cast = Some(row["input"].clone())
            }
            _ => changes.daily_external_action_cap_before_type_cast = Some(row["input"].clone()),
        }
        let input = changes.budget_cap_input(field).unwrap();
        assert_eq!(input.before_type_cast, row["before_type_cast"], "{row}");
        assert_eq!(input.value, row["value"], "{row}");
        assert_eq!(json!(input.errors), row["errors"], "{row}");
        let before =
            t.read(|c| Agent::find(c, id("bender_agent"))?.ok_or(Error::RecordNotFound("Agent")));
        let validation = t.read(|c| before.validate_changes(c, changes.clone()));
        assert_eq!(json!(validation.on(field)), row["errors"]);
        let redisplay = changes.clone();
        let result = t.try_write(move |tx| {
            Agent::find(tx.conn(), id("bender_agent"))?
                .unwrap()
                .update(tx, changes)
        });
        assert_eq!(
            redisplay.budget_cap_input(field).unwrap().before_type_cast,
            row["input"]
        );
        if !input.errors.is_empty() {
            let Error::RecordInvalid(e) = result.unwrap_err() else {
                panic!("expected raw numericality error: {row}");
            };
            assert_eq!(json!(e.on(field)), row["errors"]);
            let after = t.read(|c| {
                Agent::find(c, id("bender_agent"))?.ok_or(Error::RecordNotFound("Agent"))
            });
            assert_eq!(before.updated_at, after.updated_at);
            assert_eq!(
                (
                    before.daily_message_cap,
                    before.daily_board_post_cap,
                    before.daily_external_action_cap
                ),
                (
                    after.daily_message_cap,
                    after.daily_board_post_cap,
                    after.daily_external_action_cap
                )
            );
        } else if !input.value.is_null() && input.value.as_i64().is_none() {
            assert!(
                matches!(result, Err(Error::Other(_))),
                "Rails integer storage raises for out-of-range input"
            );
        } else {
            result.unwrap();
        }
    }
}
#[test]
fn ws11_ui_owner_signing_secret_getter_never_writes_even_blank() {
    let t = TestDb::new();
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&rails_compat::Secrets::new(
        &"ws11 public test material ".repeat(8),
    ));
    for row in gold()["secrets"].as_array().unwrap() {
        let ciphertext = row["input"].as_str().map(|s| crypto.encrypt(s));
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET webhook_signing_secret=? WHERE id=?",
                params![ciphertext, id("bender_agent")],
            )?;
            Ok(())
        });
        let expected = row.clone();
        t.write(|tx| {tx.conn().execute_batch("CREATE TEMP TRIGGER ws11_read_only_secret BEFORE UPDATE ON agents BEGIN SELECT RAISE(ABORT,'getter must not write'); END")?;Ok(())});
        t.read(|c| {
            let before:(Option<String>,Timestamp)=c.query_row("SELECT webhook_signing_secret,updated_at FROM agents WHERE id=?",[id("bender_agent")],|r|Ok((r.get(0)?,r.get(1)?)))?;
            let agent=Agent::find(c,id("bender_agent"))?.unwrap();
            let values=(0..3).map(|_|agent.webhook_signing_secret(c,&crypto)).collect::<Result<Vec<_>>>()?;
            let after:(Option<String>,Timestamp)=c.query_row("SELECT webhook_signing_secret,updated_at FROM agents WHERE id=?",[agent.id],|r|Ok((r.get(0)?,r.get(1)?)))?;
            assert_eq!(json!({"input":expected["input"],"values":values,"ciphertext_unchanged":before.0==after.0,"timestamp_unchanged":before.1==after.1}),expected);Ok(())
        });
        t.write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER ws11_read_only_secret")?;
            Ok(())
        });
    }
}
