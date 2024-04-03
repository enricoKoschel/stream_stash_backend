macro_rules! serde_struct {
    ($struct_name:ident, $($field_name:ident: $field_type:ty = $field_default:expr),+ $(,)?) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        #[serde(default)]
        struct $struct_name {
            $(
                $field_name: $field_type,
            )+
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self {
                    $(
                        $field_name: $field_default,
                    )+
                }
            }
        }
    };
    ($struct_name:ident, $($field_name:ident: $field_type:ty),+ $(,)?) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        struct $struct_name {
            $(
                $field_name: $field_type,
            )+
        }
    };
}

macro_rules! serde_enum {
    ($enum_name:ident, $($variant_name:ident($variant_type:ty)),+ $(,)?) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        #[serde(untagged)]
        enum $enum_name {
            $(
                $variant_name($variant_type),
            )+
        }
    };
}

macro_rules! log_error_location {
    ($($arg:tt)+) => {
        let msg = format!($($arg)+);
        log::error!("({}:{}) {}", line!(), column!(), msg);
    };
}

macro_rules! forbidden {
    ($($arg:tt)+) => {{
        log_error_location!($($arg)+);
        ApiError::Forbidden(())
    }};
}

macro_rules! internal_server_error {
    ($($arg:tt)+) => {{
        log_error_location!($($arg)+);
        ApiError::InternalServerError(())
    }};
}

macro_rules! parse_url {
    ($url:literal) => {
        match url::Url::parse($url) {
            Ok(val) => val,
            Err(err) => return Err(internal_server_error!("URL Parse Error: {}", err)),
        }
    };
}

macro_rules! get_json_body {
    ($req:expr, $ty:ty) => {{
        let response = match $req.send().await {
            Ok(response) => response,
            Err(err) => return Err(internal_server_error!("Reqwest error: {}", err)),
        };

        match response.json::<$ty>().await {
            Ok(json) => json,
            Err(err) => return Err(internal_server_error!("JSON deserialize error: {}", err)),
        }
    }};
}

macro_rules! add_session_cookie {
    ($jar:expr, $cookie:expr) => {
        if !crate::session::add_session_cookie($jar, &$cookie) {
            return Err(internal_server_error!("Could not add session cookie"));
        }
    };
}

pub(crate) use {
    add_session_cookie, forbidden, get_json_body, internal_server_error, log_error_location,
    parse_url, serde_enum, serde_struct,
};
