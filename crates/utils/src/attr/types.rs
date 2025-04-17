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
            /// Whether this component is combinational
            Combinational,
        };
    };
    numeric {
        pub {
            /// Scheduling goal for this component
            Schedule: "schedule",
        };
    };
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
