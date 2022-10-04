module FIFO (
    input        clock,
    input        reset,
    input        valid_up,
    output       valid_down,
    input  [7:0] I,
    output [7:0] O
);
  reg [7:0] _T;  // @[FIFO.scala 13:26]
  reg [31:0] _RAND_0;
  reg _T_1;  // @[FIFO.scala 15:27]
  reg [31:0] _RAND_1;
  assign valid_down = _T_1;  // @[FIFO.scala 16:16]
  assign O = _T;  // @[FIFO.scala 14:7]
`ifdef RANDOMIZE_GARBAGE_ASSIGN
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
  `define RANDOMIZE
`endif
`ifndef RANDOM
  `define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
  initial begin
`ifdef RANDOMIZE
`ifdef INIT_RANDOM
    `INIT_RANDOM
`endif
`ifndef VERILATOR
`ifdef RANDOMIZE_DELAY
    #`RANDOMIZE_DELAY begin
    end
`else
    #0.002 begin
    end
`endif
`endif
`ifdef RANDOMIZE_REG_INIT
    _RAND_0 = {1{`RANDOM}};
    _T = _RAND_0[7:0];
`endif  // RANDOMIZE_REG_INIT
`ifdef RANDOMIZE_REG_INIT
    _RAND_1 = {1{`RANDOM}};
    _T_1 = _RAND_1[0:0];
`endif  // RANDOMIZE_REG_INIT
`endif  // RANDOMIZE
  end  // initial
`endif  // SYNTHESIS
  always @(posedge clock) begin
    _T <= I;
    if (reset) begin
      _T_1 <= 1'h0;
    end else begin
      _T_1 <= valid_up;
    end
  end
endmodule
module InitialDelayCounter (
    input  clock,
    input  reset,
    output valid_down
);
  reg value;  // @[InitialDelayCounter.scala 8:34]
  reg [31:0] _RAND_0;
  wire _T_1;  // @[InitialDelayCounter.scala 17:17]
  wire _T_4;  // @[InitialDelayCounter.scala 17:53]
  assign _T_1 = value < 1'h1;  // @[InitialDelayCounter.scala 17:17]
  assign _T_4 = value + 1'h1;  // @[InitialDelayCounter.scala 17:53]
  assign valid_down = value;  // @[InitialDelayCounter.scala 16:16]
`ifdef RANDOMIZE_GARBAGE_ASSIGN
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
  `define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
  `define RANDOMIZE
`endif
`ifndef RANDOM
  `define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
  initial begin
`ifdef RANDOMIZE
`ifdef INIT_RANDOM
    `INIT_RANDOM
`endif
`ifndef VERILATOR
`ifdef RANDOMIZE_DELAY
    #`RANDOMIZE_DELAY begin
    end
`else
    #0.002 begin
    end
`endif
`endif
`ifdef RANDOMIZE_REG_INIT
    _RAND_0 = {1{`RANDOM}};
    value   = _RAND_0[0:0];
`endif  // RANDOMIZE_REG_INIT
`endif  // RANDOMIZE
  end  // initial
`endif  // SYNTHESIS
  always @(posedge clock) begin
    if (reset) begin
      value <= 1'h0;
    end else if (_T_1) begin
      value <= _T_4;
    end
  end
endmodule
module AtomTuple (
    input        valid_up,
    output       valid_down,
    input  [7:0] I0,
    output [7:0] O_t0b
);
  assign valid_down = valid_up;  // @[Tuple.scala 51:14]
  assign O_t0b = I0;  // @[Tuple.scala 49:9]
endmodule
module Add (
    input        valid_up,
    output       valid_down,
    input  [7:0] I_t0b,
    output [7:0] O
);
  assign valid_down = valid_up;  // @[Arithmetic.scala 108:14]
  assign O = I_t0b + 8'h5;  // @[Arithmetic.scala 106:7]
endmodule
module Module_0 (
    input        clock,
    input        reset,
    input        valid_up,
    output       valid_down,
    input  [7:0] I,
    output [7:0] O
);
  wire InitialDelayCounter_clock;  // @[Const.scala 11:33]
  wire InitialDelayCounter_reset;  // @[Const.scala 11:33]
  wire InitialDelayCounter_valid_down;  // @[Const.scala 11:33]
  wire n5_valid_up;  // @[Top.scala 18:20]
  wire n5_valid_down;  // @[Top.scala 18:20]
  wire [7:0] n5_I0;  // @[Top.scala 18:20]
  wire [7:0] n5_O_t0b;  // @[Top.scala 18:20]
  wire n6_valid_up;  // @[Top.scala 22:20]
  wire n6_valid_down;  // @[Top.scala 22:20]
  wire [7:0] n6_I_t0b;  // @[Top.scala 22:20]
  wire [7:0] n6_O;  // @[Top.scala 22:20]
  InitialDelayCounter InitialDelayCounter (  // @[Const.scala 11:33]
      .clock(InitialDelayCounter_clock),
      .reset(InitialDelayCounter_reset),
      .valid_down(InitialDelayCounter_valid_down)
  );
  AtomTuple n5 (  // @[Top.scala 18:20]
      .valid_up(n5_valid_up),
      .valid_down(n5_valid_down),
      .I0(n5_I0),
      .O_t0b(n5_O_t0b)
  );
  Add n6 (  // @[Top.scala 22:20]
      .valid_up(n6_valid_up),
      .valid_down(n6_valid_down),
      .I_t0b(n6_I_t0b),
      .O(n6_O)
  );
  assign valid_down = n6_valid_down;  // @[Top.scala 26:16]
  assign O = n6_O;  // @[Top.scala 25:7]
  assign InitialDelayCounter_clock = clock;
  assign InitialDelayCounter_reset = reset;
  assign n5_valid_up = valid_up & InitialDelayCounter_valid_down;  // @[Top.scala 21:17]
  assign n5_I0 = I;  // @[Top.scala 19:11]
  assign n6_valid_up = n5_valid_down;  // @[Top.scala 24:17]
  assign n6_I_t0b = n5_O_t0b;  // @[Top.scala 23:10]
endmodule
module MapT (
    input        clock,
    input        reset,
    input        valid_up,
    output       valid_down,
    input  [7:0] I,
    output [7:0] O
);
  wire op_clock;  // @[MapT.scala 8:20]
  wire op_reset;  // @[MapT.scala 8:20]
  wire op_valid_up;  // @[MapT.scala 8:20]
  wire op_valid_down;  // @[MapT.scala 8:20]
  wire [7:0] op_I;  // @[MapT.scala 8:20]
  wire [7:0] op_O;  // @[MapT.scala 8:20]
  Module_0 op (  // @[MapT.scala 8:20]
      .clock(op_clock),
      .reset(op_reset),
      .valid_up(op_valid_up),
      .valid_down(op_valid_down),
      .I(op_I),
      .O(op_O)
  );
  assign valid_down = op_valid_down;  // @[MapT.scala 16:16]
  assign O = op_O;  // @[MapT.scala 15:7]
  assign op_clock = clock;
  assign op_reset = reset;
  assign op_valid_up = valid_up;  // @[MapT.scala 13:17]
  assign op_I = I;  // @[MapT.scala 14:10]
endmodule
module Top (
    input        clk,
    input        reset,
    input        valid_up,
    output       valid_down,
    input  [7:0] I,
    output [7:0] O
);
  wire n1_clock;  // @[Top.scala 32:20]
  wire n1_reset;  // @[Top.scala 32:20]
  wire n1_valid_up;  // @[Top.scala 32:20]
  wire n1_valid_down;  // @[Top.scala 32:20]
  wire [7:0] n1_I;  // @[Top.scala 32:20]
  wire [7:0] n1_O;  // @[Top.scala 32:20]
  wire n7_clock;  // @[Top.scala 35:20]
  wire n7_reset;  // @[Top.scala 35:20]
  wire n7_valid_up;  // @[Top.scala 35:20]
  wire n7_valid_down;  // @[Top.scala 35:20]
  wire [7:0] n7_I;  // @[Top.scala 35:20]
  wire [7:0] n7_O;  // @[Top.scala 35:20]
  wire n8_clock;  // @[Top.scala 38:20]
  wire n8_reset;  // @[Top.scala 38:20]
  wire n8_valid_up;  // @[Top.scala 38:20]
  wire n8_valid_down;  // @[Top.scala 38:20]
  wire [7:0] n8_I;  // @[Top.scala 38:20]
  wire [7:0] n8_O;  // @[Top.scala 38:20]
  wire n9_clock;  // @[Top.scala 41:20]
  wire n9_reset;  // @[Top.scala 41:20]
  wire n9_valid_up;  // @[Top.scala 41:20]
  wire n9_valid_down;  // @[Top.scala 41:20]
  wire [7:0] n9_I;  // @[Top.scala 41:20]
  wire [7:0] n9_O;  // @[Top.scala 41:20]
  wire n10_clock;  // @[Top.scala 44:21]
  wire n10_reset;  // @[Top.scala 44:21]
  wire n10_valid_up;  // @[Top.scala 44:21]
  wire n10_valid_down;  // @[Top.scala 44:21]
  wire [7:0] n10_I;  // @[Top.scala 44:21]
  wire [7:0] n10_O;  // @[Top.scala 44:21]
  FIFO n1 (  // @[Top.scala 32:20]
      .clock(n1_clock),
      .reset(n1_reset),
      .valid_up(n1_valid_up),
      .valid_down(n1_valid_down),
      .I(n1_I),
      .O(n1_O)
  );
  MapT n7 (  // @[Top.scala 35:20]
      .clock(n7_clock),
      .reset(n7_reset),
      .valid_up(n7_valid_up),
      .valid_down(n7_valid_down),
      .I(n7_I),
      .O(n7_O)
  );
  FIFO n8 (  // @[Top.scala 38:20]
      .clock(n8_clock),
      .reset(n8_reset),
      .valid_up(n8_valid_up),
      .valid_down(n8_valid_down),
      .I(n8_I),
      .O(n8_O)
  );
  FIFO n9 (  // @[Top.scala 41:20]
      .clock(n9_clock),
      .reset(n9_reset),
      .valid_up(n9_valid_up),
      .valid_down(n9_valid_down),
      .I(n9_I),
      .O(n9_O)
  );
  FIFO n10 (  // @[Top.scala 44:21]
      .clock(n10_clock),
      .reset(n10_reset),
      .valid_up(n10_valid_up),
      .valid_down(n10_valid_down),
      .I(n10_I),
      .O(n10_O)
  );
  assign valid_down = n10_valid_down;  // @[Top.scala 48:16]
  assign O = n10_O;  // @[Top.scala 47:7]
  assign n1_clock = clk;
  assign n1_reset = reset;
  assign n1_valid_up = valid_up;  // @[Top.scala 34:17]
  assign n1_I = I;  // @[Top.scala 33:10]
  assign n7_clock = clk;
  assign n7_reset = reset;
  assign n7_valid_up = n1_valid_down;  // @[Top.scala 37:17]
  assign n7_I = n1_O;  // @[Top.scala 36:10]
  assign n8_clock = clk;
  assign n8_reset = reset;
  assign n8_valid_up = n7_valid_down;  // @[Top.scala 40:17]
  assign n8_I = n7_O;  // @[Top.scala 39:10]
  assign n9_clock = clk;
  assign n9_reset = reset;
  assign n9_valid_up = n8_valid_down;  // @[Top.scala 43:17]
  assign n9_I = n8_O;  // @[Top.scala 42:10]
  assign n10_clock = clk;
  assign n10_reset = reset;
  assign n10_valid_up = n9_valid_down;  // @[Top.scala 46:18]
  assign n10_I = n9_O;  // @[Top.scala 45:11]
endmodule
