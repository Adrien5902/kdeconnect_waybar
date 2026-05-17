use dbus::arg::RefArg;

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

        impl TryFrom<&PropMap> for $name {
            type Error = color_eyre::eyre::Error;

            fn try_from(props: &PropMap) -> Result<Self> {
                use color_eyre::eyre::eyre;
                use self::parsing::FromDbusValue;

                Ok(Self {
                    $(
                        $field: {
                            let key = dbus_struct!(@camel stringify!($field));

                            let value = props
                                .get(key)
                                .ok_or_else(|| eyre!("missing property: {}", key))?;

                            <$ty as FromDbusValue>::from_dbus(&*value.0)
                                .ok_or_else(|| eyre!("invalid property type: {}", key))?
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
