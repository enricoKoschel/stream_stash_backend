// TODO: Remove?
macro_rules! serde_struct {
    ($vis:vis $struct_name:ident, $($field_name:ident: $field_type:ty = $field_default:expr),+ $(,)?) => {
        #[allow(non_snake_case)]
        #[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
        #[serde(default)]
        $vis struct $struct_name {
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
    ($vis:vis $struct_name:ident, $($field_name:ident: $field_type:ty),+ $(,)?) => {
        #[allow(non_snake_case)]
        #[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
        $vis struct $struct_name {
            $(
                $field_name: $field_type,
            )+
        }
    };
}

macro_rules! log_error_location {
    ($($arg:tt)+) => {
        log::error!("[{}] {}", crate::macros::error_context!(), format!($($arg)+));
    };
}

macro_rules! forbidden {
    ($($arg:tt)+) => {{
        crate::ApiError::Forbidden(format!($($arg)+), crate::macros::error_context!())
    }};
}

macro_rules! internal_server_error {
    ($($arg:tt)+) => {{
        crate::ApiError::InternalServerError(format!($($arg)+), crate::macros::error_context!())
    }};
}

macro_rules! parse_url {
    ($url:expr) => {
        url::Url::parse(&$url)
            .map_err(|err| crate::macros::internal_server_error!("URL Parse Error: {}", err))
    };
}

macro_rules! get_json_body {
    ($req:expr, $ty:ty) => {
        'block: {
            let response = match $req.send().await {
                Ok(response) => response,
                Err(err) => {
                    break 'block Err(crate::macros::internal_server_error!(
                        "Reqwest error: {}",
                        err
                    ))
                }
            };

            let json = match response.json::<serde_json::Value>().await {
                Ok(json) => json,
                Err(err) => {
                    break 'block Err(crate::macros::internal_server_error!(
                        "JSON deserialize error: {}",
                        err
                    ))
                }
            };

            Ok(serde_json::from_value::<$ty>(json.clone()).map_err(|_| json))
        }
    };
}

macro_rules! get_text_body {
    ($req:expr) => {
        'block: {
            let response = match $req.send().await {
                Ok(response) => response,
                Err(err) => {
                    break 'block Err(crate::macros::internal_server_error!(
                        "Reqwest error: {}",
                        err
                    ))
                }
            };

            response.text().await.map_err(|err| {
                crate::macros::internal_server_error!("Text deserialize error: {}", err)
            })
        }
    };
}

macro_rules! add_session_cookie {
    ($jar:expr, $cookie:expr) => {
        if crate::session::add_session_cookie($jar, &$cookie) {
            Ok(())
        } else {
            Err(crate::macros::internal_server_error!(
                "Could not add session cookie"
            ))
        }
    };
}

macro_rules! error_context {
    () => {{
        crate::ErrorContext {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    }};
}

macro_rules! compare_scope {
    ($scope:expr) => {
        if crate::google::compare_scope($scope) {
            Ok(())
        } else {
            Err(crate::macros::forbidden!(
                "Requested and received scope not the same"
            ))
        }
    };
}

pub(crate) use {
    add_session_cookie, compare_scope, error_context, forbidden, get_json_body, get_text_body,
    internal_server_error, log_error_location, parse_url, serde_struct,
};
