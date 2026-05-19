use dbus::arg::{PropMap, RefArg};

use crate::Result;
pub trait FromDbusMap: Sized {
    fn from_props(props: PropMap) -> Result<Self>;
}

pub trait FromDbusValue: Sized {
    fn from_dbus(v: &dyn RefArg) -> Option<Self>;
}

impl FromDbusValue for bool {
    fn from_dbus(v: &dyn RefArg) -> Option<Self> {
        v.as_i64().map(|v| v != 0)
    }
}

impl FromDbusValue for i64 {
    fn from_dbus(v: &dyn RefArg) -> Option<Self> {
        v.as_i64().map(|v| v as i64)
    }
}

impl FromDbusValue for String {
    fn from_dbus(v: &dyn RefArg) -> Option<Self> {
        v.as_str().map(|v| v.to_string())
    }
}

impl<T> FromDbusValue for Vec<T>
where
    T: FromDbusValue,
{
    fn from_dbus(v: &dyn RefArg) -> Option<Vec<T>> {
        v.as_iter()
            .map(|iter| iter.map(|v| T::from_dbus(v)).collect::<Option<Vec<T>>>())
            .flatten()
    }
}

macro_rules! dbus_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                pub $field: $ty,
            )*
        }

        impl FromDbusMap for $name {
            fn from_props(props: PropMap) -> Result<Self> {
                use crate::error::Error;
                use crate::parsing::FromDbusValue;

                Ok(Self {
                    $(
                        $field: {
                            let key = dbus_struct!(@camel stringify!($field));

                            let value = props
                                .get(key)
                                .ok_or_else(|| Error::DBusParsingFail(format!("missing property: {}", key)))?;

                            <$ty as FromDbusValue>::from_dbus(&*value.0)
                                .ok_or_else(|| Error::DBusParsingFail(format!("invalid property type: {}", key)))?
                        },
                    )*
                })
            }
        }
    };

    // snake_case → camelCase
    (@camel $s:expr) => {{
        let s = $s;

        let mut out = String::new();
        let mut upper = false;

        for c in s.chars() {
            if c == '_' {
                upper = true;
            } else if upper {
                out.push(c.to_ascii_uppercase());
                upper = false;
            } else {
                out.push(c);
            }
        }

        Box::leak(out.into_boxed_str())
    }};
}

macro_rules! dbus_enum {
    (
        $vis:vis enum $name:ident {
            $(
                $variant:ident
            ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $(
                $variant,
            )*
        }

        impl std::str::FromStr for $name {
            type Err = crate::error::Error;

            fn from_str(s: &str) -> Result<Self> {
                $(
                    if s == dbus_enum!(@to_snake stringify!($variant)) {
                        return Ok(Self::$variant);
                    }
                )*

                Err(Self::Err::DBusParsingFail(format!(
                    "invalid {} variant: {}",
                    stringify!($name),
                    s
                )))
            }
        }

        impl crate::parsing::FromDbusValue for $name {
            fn from_dbus(v: &dyn dbus::arg::RefArg) -> Option<Self> {
                v.as_str()?.parse().ok()
            }
        }
    };

    (@to_snake $s:expr) => {{
        let s = $s;
        let mut out = String::new();

        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i != 0 {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }

        Box::leak(out.into_boxed_str())
    }};
}
