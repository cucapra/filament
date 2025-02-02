use crate::attr_set;

attr_set! {
    comp_attrs;
    flag {
        pub {
            /// This is a toplevel component
            TopLevel: "toplevel",
            /// Use a counter based FSM design
            CounterFSM: "counter_fsm",
            /// Whether to schedule the component
            Schedule: "schedule",
        };
    };
    numeric {};
    float {};
}

attr_set! {
    port_attrs;
    flag {};
    numeric {};
    float {
        priv {
            /// Combinational delay of this port
            CombDelay,
        };
    };
}
