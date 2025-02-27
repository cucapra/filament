#[macro_export]
/*
A macro to create a single attribute enum. Automatically derives [FromString] for the enum using the [strum] crate. Supports private flags that cannot be derived from a string.

Example usage:
```rust
attr_enum! {
  enum Bool;
  pub {
      /// This is a toplevel component
      TopLevel: "top_level",
      /// Use a counter based FSM design
      CounterFSM: "counter_fsm",
  };
  priv {
      /// this is a private flag
      HiddenFlag,
  };
}
```
*/
macro_rules! attr_enum {
  (
      enum $name:ident;
      pub {
          $(
              $(#[$pub_meta:meta])*
              $pub:ident: $pub_str:literal,
          )*
      };
      priv {
          $(
              $(#[$priv_meta:meta])*
              $priv:ident,
          )*
      };
  ) => {
      #[derive(Clone, Copy, PartialEq, strum_macros::EnumString, Debug, Eq, Hash)]
      pub enum $name {
          $(
              $(#[$pub_meta])*
              #[strum(serialize = $pub_str)]
              $pub,
          )*
          $(
              $(#[$priv_meta])*
              #[strum(disabled)]
              $priv,
          )*
      }

      impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$pub => write!(f, "{}", $pub_str),
                    )*
                    $(
                        Self::$priv => write!(f, "{}", stringify!($priv)),
                    )*
                    // Unreachable pattern here to handle the case where the enum
                    // has no variants
                    #[allow(unreachable_patterns)]
                    _ => write!(f, "{}", stringify!($name)),
                }
            }
      }
  };
  (enum $name:ident;)  => {
      $crate::attr_enum! {
          enum $name;
          pub {};
          priv {};
      }
  };
  (enum $name:ident;
      pub {
          $(
              $(#[$pub_meta:meta])*
              $pub:ident: $pub_str:literal,
          )*
      };
  ) => {
    $crate::attr_enum! {
          enum $name;
          pub {
              $(
                  $(#[$pub_meta])*
                  $pub: $pub_str,
              )*
          };
          priv {};
      }
  };
  (enum $name:ident;
      priv {
          $(
              $(#[$priv_meta:meta])*
              $priv:ident,
          )*
      };
  ) => {
      $crate::attr_enum! {
          enum $name;
          pub {};
          priv {
              $(
                  $(#[$priv_meta])*
                  $priv,
              )*
          };
      }
  };
}

#[macro_export]
/*
We can define a set of attributes as follows:
```rust
attr_set! {
  set_name;
  bool {
  pub:
      /// This is a toplevel component
      TopLevel: "top_level",
      /// Use a counter based FSM design
      CounterFSM: "counter_fsm",

  priv:
      /// this is a private flag
      HiddenFlag,
  };
  num {
  pub:
      /// Example numerical attribute
      ExampleNum: "example_num",
  priv:
      /// Private numerical attribute
      HiddenNum,
  };
}
*/
macro_rules! attr_set {
    (
        $module:ident;
        flag {
            $(
                $flag_tokens:tt
            )*
        };
        numeric {
            $(
                $num_tokens:tt
            )*
        };
        float {
            $(
                $float_tokens:tt
            )*
        };
    ) => {
        pub mod $module {
            $crate::attr_enum! {
                enum Bool;
                $($flag_tokens)*
            }

            $crate::attr_enum! {
                enum Num;
                $($num_tokens)*
            }

            $crate::attr_enum! {
                enum Float;
                $($float_tokens)*
            }

            pub type Attrs = $crate::Attributes<Bool, Num, Float>;
        }
    };
}
