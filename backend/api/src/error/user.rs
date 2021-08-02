use super::{ise, BasicError, Service, ServiceSession};
use actix_http::Error;
use http::StatusCode;
use paperclip::actix::api_v2_errors;

#[api_v2_errors(
    code = 401,
    code = 403,
    code = 404,
    description = "Not Found: User not Found",
    code = 500
)]
pub enum NotFound {
    UserNotFound,
    InternalServerError(anyhow::Error),
}

impl<T: Into<anyhow::Error>> From<T> for NotFound {
    fn from(e: T) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl Into<actix_web::Error> for NotFound {
    fn into(self) -> actix_web::Error {
        match self {
            Self::UserNotFound => {
                BasicError::with_message(StatusCode::NOT_FOUND, "User Not Found".to_owned()).into()
            }
            Self::InternalServerError(e) => ise(e),
        }
    }
}

#[api_v2_errors(
    code = 400,
    code = 404,
    code = 409,
    description = "Conflict: Another user with the provided username already exists",
    code = 420,
    description = "Unprocessable Entity: No username or email was provided",
    code = 500,
    code = 501
)]
pub enum Update {
    InternalServerError(anyhow::Error),
    Username(Username),
    UserNotFound,
}

impl<T: Into<anyhow::Error>> From<T> for Update {
    fn from(e: T) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl Into<actix_web::Error> for Update {
    fn into(self) -> Error {
        match self {
            Self::InternalServerError(e) => ise(e),
            Self::Username(e) => match e {
                Username::Empty => BasicError::with_message(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Empty Username Provided".to_owned(),
                )
                .into(),
                Username::Taken => {
                    BasicError::with_message(StatusCode::CONFLICT, "Username Is Taken".to_owned())
                        .into()
                }
            },
            Self::UserNotFound => {
                BasicError::with_message(StatusCode::NOT_FOUND, "User Not Found".to_owned()).into()
            }
        }
    }
}

#[api_v2_errors(
    code = 400,
    code = 409,
    description = "Conflict: Another user with the provided username or email already exists",
    code = 420,
    description = "Unprocessable Entity: No username or email was provided",
    code = 500,
    code = 501
)]
pub enum Register {
    InternalServerError(anyhow::Error),
    Username(Username),
    VerifyEmail(VerifyEmail),
    Service(Service),
}

impl<T: Into<anyhow::Error>> From<T> for Register {
    fn from(e: T) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl Into<actix_web::Error> for Register {
    fn into(self) -> actix_web::Error {
        match self {
            Self::InternalServerError(e) => ise(e),
            Self::Username(Username::Empty) => BasicError::with_message(
                StatusCode::UNPROCESSABLE_ENTITY,
                "No username was provided".to_owned(),
            )
            .into(),
            Self::Username(Username::Taken) => {
                BasicError::with_message(StatusCode::CONFLICT, "Username already taken".to_owned())
                    .into()
            }
            Self::VerifyEmail(e) => e.into(),
            Self::Service(e) => e.into(),
        }
    }
}

impl From<VerifyEmail> for Register {
    fn from(err: VerifyEmail) -> Self {
        Self::VerifyEmail(err)
    }
}

impl From<Service> for Register {
    fn from(err: Service) -> Self {
        Self::Service(err)
    }
}

impl From<Email> for Register {
    fn from(err: Email) -> Self {
        Self::VerifyEmail(err.into())
    }
}

pub enum Username {
    Empty,
    Taken,
}

pub enum Email {
    TakenBasic,
    TakenGoogle,
    Empty,
}

#[api_v2_errors(
    code = 400,
    code = 409,
    description = "Conflict: Another user with the provided email already exists",
    code = 420,
    description = "Unprocessable Entity: No email was provided",
    code = 500,
    code = 501
)]
pub enum VerifyEmail {
    InternalServerError(anyhow::Error),
    Email(Email),
    ServiceSession(ServiceSession),
}

impl<T: Into<anyhow::Error>> From<T> for VerifyEmail {
    fn from(e: T) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl Into<actix_web::Error> for VerifyEmail {
    fn into(self) -> actix_web::Error {
        match self {
            Self::InternalServerError(e) => ise(e),
            Self::Email(e) => match e {
                Email::TakenBasic | Email::TakenGoogle => BasicError::with_message(
                    StatusCode::CONFLICT,
                    "A user with this email already exists".to_owned(),
                )
                .into(),
                Email::Empty => BasicError::with_message(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "No email address was provided".to_owned(),
                )
                .into(),
            },
            Self::ServiceSession(e) => e.into(),
        }
    }
}

impl From<ServiceSession> for VerifyEmail {
    fn from(err: ServiceSession) -> Self {
        Self::ServiceSession(err)
    }
}

impl From<Email> for VerifyEmail {
    fn from(err: Email) -> Self {
        Self::Email(err)
    }
}