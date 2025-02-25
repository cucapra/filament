use crate::attr_set;

attr_set! {
    comp_attrs;
    flag {
        pub {
            /// This is a toplevel component
            TopLevel: "toplevel",
            /// Use a counter based FSM design
            CounterFSM: "counter_fsm",
        };
        priv {
            /// This component is monomorphic
            Monomorphic,
            /// Bundles have been eliminated
            BundleLess,
        };
    };
    numeric {};
    float {};
}

attr_set! {
    port_attrs;
    flag {};
    numeric {};
    float {};
}
