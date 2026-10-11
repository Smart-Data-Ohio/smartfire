//! `FirstRunsController` (reference/app/controllers/first_runs_controller.rb): set up the account
//! and its first administrator. The retained form and the SPA's JSON contract
//! (`/api/v1/first_run`) share the before-actions and the setup itself; only how a submission
//! is read and how the answer is written differ.

use campfire_api_types::{FirstRunState, FirstRunSubmission};
use campfire_db::{Account, FirstRun, PasswordDigest};
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};
use campfire_people::controllers::auth::{self, ResponseMode};
use campfire_retained::first_runs;

use super::presenters;
use super::presenters::attachments::{self, Assignment, Record};
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::page::retained_page;

/// `allow_unauthenticated_access`, `before_action :prevent_repeats`
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    prevent_repeats(c).await?;
    c.respond_to(&[&format::HTML])?;
    retained_page!(c, StatusCode::OK, |ctx| first_runs::Show { ctx }).await
}

pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    prevent_repeats(c).await?;

    // params.require(:user).permit(:name, :avatar, :email_address, :password)
    let user = c
        .params
        .require("user")?
        .permit(&campfire_kit::permit_keys(&[
            "name",
            "avatar",
            "email_address",
            "password",
        ]));
    let submission = Submission {
        name: user.get("name").and_then(|p| p.to_s()),
        email_address: user.get("email_address").and_then(|p| p.to_s()),
        password: user.get("password").and_then(|p| p.to_s()),
        avatar: Assignment::from_params(&user, "avatar")?,
    };
    set_up(c, submission, ResponseMode::Html).await
}

/// `GET /api/v1/first_run`: the retained page's before-actions, answered as JSON. Once the
/// account exists, `prevent_repeats`' redirect becomes a `navigate` home.
pub async fn show_json(c: &mut Ctx) -> Result {
    let result = show_json_response(c).await;
    auth::complete(c, result)
}

async fn show_json_response(c: &mut Ctx) -> Result {
    auth::before_actions(
        c,
        "first_runs#show",
        Before::default().allow_unauthenticated_access(),
        ResponseMode::Json,
    )
    .await?;
    prevent_repeats(c).await?;
    let csrf_token = c.authenticity_tokens().global();
    auth::json(c, StatusCode::OK, &FirstRunState::Pending { csrf_token })
}

/// `POST /api/v1/first_run`: a [`FirstRunSubmission`] as the JSON body, or with an avatar as
/// `multipart/form-data` (the JSON in `submission`, the picture in `avatar`). The forgery check
/// takes the header or body token, as the other signed-out contracts do.
pub async fn create_json(c: &mut Ctx) -> Result {
    let result = create_json_response(c).await;
    auth::complete(c, result)
}

async fn create_json_response(c: &mut Ctx) -> Result {
    auth::before_actions(
        c,
        "first_runs#create",
        Before::default().allow_unauthenticated_access(),
        ResponseMode::Json,
    )
    .await?;
    prevent_repeats(c).await?;

    let (body, avatar) = if c.request.media_type().as_deref() == Some("multipart/form-data") {
        let json = c
            .params
            .get("submission")
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_owned();
        let body: FirstRunSubmission = auth::decode(c, json.as_bytes())?;
        let avatar = Assignment::from_params(&c.params, "avatar")?;
        (body, avatar)
    } else {
        (auth::body(c).await?, Assignment::Unchanged)
    };
    // Only a picture uploaded with the form. A signed blob id needs a signed-in direct upload,
    // and the retained form would only fail on one after creating the account.
    if matches!(avatar, Assignment::Signed(_) | Assignment::Invalid) {
        return auth::field_error(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "avatar",
            "The avatar isn't an uploaded picture.",
        );
    }
    let submission = Submission {
        name: Some(body.name),
        email_address: body.email_address,
        password: body.password,
        avatar,
    };
    set_up(c, submission, ResponseMode::Json).await
}

/// The permitted `user` params, however they arrived.
struct Submission {
    name: Option<String>,
    email_address: Option<String>,
    password: Option<String>,
    avatar: Assignment,
}

/// `FirstRun.create!(user_params)`, `start_new_session_for`, `redirect_to root_url`.
async fn set_up(c: &mut Ctx, submission: Submission, mode: ResponseMode) -> Result {
    let Submission {
        name,
        email_address,
        password,
        avatar,
    } = submission;
    let password = password.filter(|s| !s.is_empty());
    // Rails commits Account.create! before building/saving the room and its creator.
    // A later failure leaves the account present, so subsequent first runs redirect.
    match c
        .app()
        .db
        .write(|tx| Account::create(tx, FirstRun::ACCOUNT_NAME))
        .await
    {
        Ok(_) => {}
        Err(error) if presenters::accounts::is_record_not_unique(&error) => {
            let root = c.url_for(&campfire_routes::root());
            return mode.navigate(c, &root);
        }
        Err(error) => return Err(Error::internal(error)),
    }
    let Some(name) = name else {
        return Err(Error::internal(anyhow::anyhow!(
            "NOT NULL constraint failed: users.name"
        )));
    };
    let avatar = avatar.stage(c.app()).await?;
    let password_digest = match password {
        Some(password) => Some(
            PasswordDigest::hash(password, c.app().db.env().bcrypt_cost)
                .await
                .map_err(Error::internal)?,
        ),
        None => None,
    };

    let result = c
        .app()
        .db
        .write(move |tx| {
            let administrator = FirstRun::create_administrator(
                tx,
                &name,
                email_address.as_deref(),
                password_digest,
            )?;
            attachments::assign(tx, Record::user(administrator.id), "avatar", avatar)?;
            Ok(administrator)
        })
        .await;

    let root = c.url_for(&campfire_routes::root());
    match result {
        Ok(administrator) => {
            concerns::start_new_session_for(c, administrator).await?;
            mode.signed_in(c, &root)
        }
        // rescue ActiveRecord::RecordNotUnique
        Err(error) if presenters::accounts::is_record_not_unique(&error) => mode.navigate(c, &root),
        Err(error) => Err(Error::internal(error)),
    }
}

/// `redirect_to root_url if Account.any?`. The SPA's `/app/first_run` shell runs it too.
pub(crate) async fn prevent_repeats(c: &mut Ctx) -> Result<()> {
    let any = c
        .app()
        .db
        .read(|conn| Ok(Account::count(conn)? > 0))
        .await
        .map_err(Error::internal)?;
    if any {
        let root = c.url_for(&campfire_routes::root());
        return halt(c.redirect_to(&root)?);
    }
    Ok(())
}
