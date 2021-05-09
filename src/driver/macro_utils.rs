macro_rules! enum_str {
    (pub enum $name:ident {
        $($variant:ident = $val:expr),*,
    }) => {
        pub enum $name {
            $($variant = $val),*
        }

        impl $name {
            fn name(&self) -> &'static str {
                match self {
                    $($name::$variant => stringify!($variant)),*
                }
            }
        }
    };
}
