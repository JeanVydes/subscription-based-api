#[derive(Debug)]
#[allow(dead_code)]
pub enum APIMessages{
    // Generic Errors
    InternalServerError,
    BadRequest,
    Unauthorized,
    NotFound,
    Forbidden,
    Conflict,
    UnprocessableEntity,
    TooManyRequests,
    ServiceUnavailable,
    GatewayTimeout,
    // Token
    Token(TokenMessages),
    // Generic
    Input(InputMessages),
    // Email
    Email(EmailMessages),
    // Storage
    Mongo(MongoMessages),
    Redis(RedisMessages),
    // Customer
    Customer(CustomerMessages),
}

#[derive(Debug)]
pub enum TokenMessages {
    Missing,
    NotSigningKeyFound,
    Created,
    ErrorCreating,
    Expired,
    ErrorValidating,
    Renewed,
    ErrorRenewing,

    NotAllowedScopesToPerformAction,

    OnlyLegacyProvider,
    OnlyGoogleProvider,

    ErrorFetchingUserFromGoogle,
    ErrorRequestingGoogleToken,

    NotAuthorizationHeader,
    ErrorParsingToken,
}

#[derive(Debug)]
pub enum InputMessages {
    InvalidNameLength,
    InvalidOldPasswordLength,
    InvalidNewPasswordLength,
    PasswordMustHaveAtLeastOneLetterAndOneNumber,
    NewPasswordAndOldPasswordMustBeDifferent,
    NewPasswordConfirmationMustMatch,
}

#[derive(Debug)]
pub enum CustomerMessages {
    Created,
    Found,
    NotFound,
    NotAcceptedTerms,
    
    InvalidType,

    PasswordConfirmationDoesNotMatch,
    IncorrectPassword,
    ErrorVerifyingPassword,
    ErrorHashingPassword,

    ErrorRegisteringCustomerInMarketingPlatform,

    NameUpdated,
    PasswordUpdated,
    EmailAdded,

    NotFoundByID,
}

#[derive(Debug)]
pub enum MongoMessages {
    ErrorInserting,
}

#[derive(Debug)]
pub enum RedisMessages {
    FailedToConnect,
    ErrorFetching,
    ErrorDeleting,
    ErrorSettingKey,
}

#[derive(Debug)]
pub enum EmailMessages {
    Verified,
    Invalid,

    Taken,
    TakenByOtherCustomer,
    TakenByYou,

    EmailAndPasswordMustBeDifferent,
    ErrorSendingVerificationEmail,
    MaxEmailsReached,
}

impl ToString for APIMessages {
    fn to_string(&self) -> String {
        match self {
            APIMessages::InternalServerError => "generic.internal_server_error".to_string(),
            APIMessages::BadRequest => "generic.bad_request".to_string(),
            APIMessages::Unauthorized => "generic.unauthorized".to_string(),
            APIMessages::NotFound => "generic.not_found".to_string(),
            APIMessages::Forbidden => "generic.forbidden".to_string(),
            APIMessages::Conflict => "generic.conflict".to_string(),
            APIMessages::UnprocessableEntity => "generic.unprocessable_entity".to_string(),
            APIMessages::TooManyRequests => "generic.too_many_requests".to_string(),
            APIMessages::ServiceUnavailable => "generic.service_unavailable".to_string(),
            APIMessages::GatewayTimeout => "generic.gateway_timeout".to_string(),
            APIMessages::Token(token_message) => token_message.to_string(),
            APIMessages::Input(input_message) => input_message.to_string(),
            APIMessages::Email(email_message) => email_message.to_string(),
            APIMessages::Mongo(mongo_message) => mongo_message.to_string(),
            APIMessages::Redis(redis_message) => redis_message.to_string(),
            APIMessages::Customer(customer_message) => customer_message.to_string(),
        }
    }
}

impl ToString for TokenMessages {
    fn to_string(&self) -> String {
        match self {
            TokenMessages::Missing => "token.missing".to_string(),
            TokenMessages::NotSigningKeyFound => "token.not_signing_key_found".to_string(),
            TokenMessages::Created => "token.created".to_string(),
            TokenMessages::ErrorCreating => "token.error_creating".to_string(),
            TokenMessages::Expired => "token.expired".to_string(),
            TokenMessages::ErrorValidating => "token.error_validating".to_string(),
            TokenMessages::Renewed => "token.renewed".to_string(),
            TokenMessages::ErrorRenewing => "token.error_renewing".to_string(),
            TokenMessages::OnlyLegacyProvider => "token.only_legacy_provider".to_string(),
            TokenMessages::OnlyGoogleProvider => "token.only_google_provider".to_string(),
            TokenMessages::ErrorFetchingUserFromGoogle => "token.error_fetching_user_from_google".to_string(),
            TokenMessages::ErrorRequestingGoogleToken => "token.error_requesting_google_token".to_string(),
            TokenMessages::NotAuthorizationHeader => "token.not_authorization_header".to_string(),
            TokenMessages::ErrorParsingToken => "token.error_parsing_token".to_string(),
            TokenMessages::NotAllowedScopesToPerformAction => "token.not_allowed_scopes_to_perform_action".to_string(),
        }
    }
}

impl ToString for InputMessages {
    fn to_string(&self) -> String {
        match self {
            InputMessages::InvalidNameLength => "generic.invalid_name_length".to_string(),
            InputMessages::InvalidOldPasswordLength => "generic.invalid_old_password_length".to_string(),
            InputMessages::InvalidNewPasswordLength => "generic.invalid_new_password_length".to_string(),
            InputMessages::NewPasswordAndOldPasswordMustBeDifferent => {
                "generic.new_password_and_old_password_must_be_different".to_string()
            }
            InputMessages::NewPasswordConfirmationMustMatch => {
                "generic.new_password_confirmation_must_match".to_string()
            },
            InputMessages::PasswordMustHaveAtLeastOneLetterAndOneNumber => {
                "generic.password_must_have_at_least_one_letter_and_one_number".to_string()
            },
        }
    }
}

impl ToString for CustomerMessages {
    fn to_string(&self) -> String {
        match self {
            CustomerMessages::Created => "customer.created".to_string(),
            CustomerMessages::Found => "customer.found".to_string(),
            CustomerMessages::NotFound => "customer.not_found".to_string(),
            CustomerMessages::NotAcceptedTerms => "customer.not_accepted_terms".to_string(),
            CustomerMessages::PasswordConfirmationDoesNotMatch => {
                "customer.password_confirmation_does_not_match".to_string()
            }
            CustomerMessages::IncorrectPassword => "customer.incorrect_password".to_string(),
            CustomerMessages::ErrorVerifyingPassword => "customer.error_verifying_password".to_string(),
            CustomerMessages::ErrorHashingPassword => "customer.error_hashing_password".to_string(),
            CustomerMessages::ErrorRegisteringCustomerInMarketingPlatform => {
                "customer.error_registering_in_marketing_platform".to_string()
            }
            CustomerMessages::NameUpdated => "customer.name_updated".to_string(),
            CustomerMessages::PasswordUpdated => "customer.password_updated".to_string(),
            CustomerMessages::EmailAdded => "customer.email_added".to_string(),
            CustomerMessages::InvalidType => "customer.invalid_type".to_string(),
            CustomerMessages::NotFoundByID => "customer.not_found_by_id".to_string(),
        }
    }
}

impl ToString for MongoMessages {
    fn to_string(&self) -> String {
        match self {
            MongoMessages::ErrorInserting => "storage.mongo_error_inserting".to_string(),
        }
    }
}

impl ToString for RedisMessages {
    fn to_string(&self) -> String {
        match self {
            RedisMessages::FailedToConnect => "storage.redis_failed_to_connect".to_string(),
            RedisMessages::ErrorFetching => "storage.redis_error_fetching".to_string(),
            RedisMessages::ErrorDeleting => "storage.redis_error_deleting".to_string(),
            RedisMessages::ErrorSettingKey => "storage.redis_error_setting_key".to_string(),
        }
    }
}

impl ToString for EmailMessages {
    fn to_string(&self) -> String {
        match self {
            EmailMessages::Verified => "email.verified".to_string(),
            EmailMessages::Invalid => "email.invalid".to_string(),
            EmailMessages::Taken => "email.taken".to_string(),
            EmailMessages::TakenByOtherCustomer => "email.taken_by_other_customer".to_string(),
            EmailMessages::TakenByYou => "email.taken_by_you".to_string(),
            EmailMessages::EmailAndPasswordMustBeDifferent => {
                "email.and_password_must_be_different".to_string()
            }
            EmailMessages::ErrorSendingVerificationEmail => {
                "email.error_sending_verification_email".to_string()
            }
            EmailMessages::MaxEmailsReached => "email.max_emails_reached".to_string(),
        }
    }
}
