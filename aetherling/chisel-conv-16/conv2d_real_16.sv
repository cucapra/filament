module FIFO(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  output [7:0] O_0
);
  reg [7:0] _T_0; // @[FIFO.scala 13:26]
  reg [31:0] _RAND_0;
  reg  _T_1; // @[FIFO.scala 15:27]
  reg [31:0] _RAND_1;
  assign valid_down = _T_1; // @[FIFO.scala 16:16]
  assign O_0 = _T_0; // @[FIFO.scala 14:7]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T_0 = _RAND_0[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_1 = _RAND_1[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    _T_0 <= I_0;
    if (reset) begin
      _T_1 <= 1'h0;
    end else begin
      _T_1 <= valid_up;
    end
  end
endmodule
module NestedCounters(
  input   CE,
  output  valid
);
  assign valid = CE; // @[NestedCounters.scala 65:13]
endmodule
module NestedCounters_1(
  input   CE,
  output  valid
);
  wire  NestedCounters_CE; // @[NestedCounters.scala 53:31]
  wire  NestedCounters_valid; // @[NestedCounters.scala 53:31]
  NestedCounters NestedCounters ( // @[NestedCounters.scala 53:31]
    .CE(NestedCounters_CE),
    .valid(NestedCounters_valid)
  );
  assign valid = NestedCounters_valid; // @[NestedCounters.scala 56:11]
  assign NestedCounters_CE = CE; // @[NestedCounters.scala 57:22]
endmodule
module NestedCountersWithNumValid(
  input   CE,
  output  valid
);
  wire  NestedCounters_CE; // @[NestedCounters.scala 20:44]
  wire  NestedCounters_valid; // @[NestedCounters.scala 20:44]
  NestedCounters_1 NestedCounters ( // @[NestedCounters.scala 20:44]
    .CE(NestedCounters_CE),
    .valid(NestedCounters_valid)
  );
  assign valid = NestedCounters_valid; // @[NestedCounters.scala 22:9]
  assign NestedCounters_CE = CE; // @[NestedCounters.scala 21:27]
endmodule
module RAM_ST(
  input        clk,
  input        RE,
  input  [1:0] RADDR,
  output [7:0] RDATA_0,
  input        WE,
  input  [1:0] WADDR,
  input  [7:0] WDATA_0
);
  wire  write_elem_counter_CE; // @[RAM_ST.scala 20:34]
  wire  write_elem_counter_valid; // @[RAM_ST.scala 20:34]
  wire  read_elem_counter_CE; // @[RAM_ST.scala 21:33]
  wire  read_elem_counter_valid; // @[RAM_ST.scala 21:33]
  reg [7:0] ram [0:3]; // @[RAM_ST.scala 29:24]
  reg [31:0] _RAND_0;
  wire [7:0] ram__T_8_data; // @[RAM_ST.scala 29:24]
  wire [1:0] ram__T_8_addr; // @[RAM_ST.scala 29:24]
  wire [7:0] ram__T_2_data; // @[RAM_ST.scala 29:24]
  wire [1:0] ram__T_2_addr; // @[RAM_ST.scala 29:24]
  wire  ram__T_2_mask; // @[RAM_ST.scala 29:24]
  wire  ram__T_2_en; // @[RAM_ST.scala 29:24]
  reg  ram__T_8_en_pipe_0;
  reg [31:0] _RAND_1;
  reg [1:0] ram__T_8_addr_pipe_0;
  reg [31:0] _RAND_2;
  wire [1:0] _GEN_1 = 2'h1 == WADDR ? 2'h1 : 2'h0; // @[RAM_ST.scala 31:71]
  wire [1:0] _GEN_2 = 2'h2 == WADDR ? 2'h2 : _GEN_1; // @[RAM_ST.scala 31:71]
  wire [1:0] _GEN_3 = 2'h3 == WADDR ? 2'h3 : _GEN_2; // @[RAM_ST.scala 31:71]
  wire [2:0] _T = {{1'd0}, _GEN_3}; // @[RAM_ST.scala 31:71]
  wire [1:0] _GEN_10 = 2'h1 == RADDR ? 2'h1 : 2'h0; // @[RAM_ST.scala 32:46]
  wire [1:0] _GEN_11 = 2'h2 == RADDR ? 2'h2 : _GEN_10; // @[RAM_ST.scala 32:46]
  wire [1:0] _GEN_12 = 2'h3 == RADDR ? 2'h3 : _GEN_11; // @[RAM_ST.scala 32:46]
  wire [2:0] _T_3 = {{1'd0}, _GEN_12}; // @[RAM_ST.scala 32:46]
  NestedCountersWithNumValid write_elem_counter ( // @[RAM_ST.scala 20:34]
    .CE(write_elem_counter_CE),
    .valid(write_elem_counter_valid)
  );
  NestedCountersWithNumValid read_elem_counter ( // @[RAM_ST.scala 21:33]
    .CE(read_elem_counter_CE),
    .valid(read_elem_counter_valid)
  );
  assign ram__T_8_addr = ram__T_8_addr_pipe_0;
  assign ram__T_8_data = ram[ram__T_8_addr]; // @[RAM_ST.scala 29:24]
  assign ram__T_2_data = WDATA_0;
  assign ram__T_2_addr = _T[1:0];
  assign ram__T_2_mask = 1'h1;
  assign ram__T_2_en = write_elem_counter_valid;
  assign RDATA_0 = ram__T_8_data; // @[RAM_ST.scala 32:9]
  assign write_elem_counter_CE = WE; // @[RAM_ST.scala 23:25]
  assign read_elem_counter_CE = RE; // @[RAM_ST.scala 24:24]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  _RAND_0 = {1{`RANDOM}};
  `ifdef RANDOMIZE_MEM_INIT
  for (initvar = 0; initvar < 4; initvar = initvar+1)
    ram[initvar] = _RAND_0[7:0];
  `endif // RANDOMIZE_MEM_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  ram__T_8_en_pipe_0 = _RAND_1[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  ram__T_8_addr_pipe_0 = _RAND_2[1:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if(ram__T_2_en & ram__T_2_mask) begin
      ram[ram__T_2_addr] <= ram__T_2_data; // @[RAM_ST.scala 29:24]
    end
    ram__T_8_en_pipe_0 <= read_elem_counter_valid;
    if (read_elem_counter_valid) begin
      ram__T_8_addr_pipe_0 <= _T_3[1:0];
    end
  end
endmodule
module ShiftT(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  output [7:0] O_0
);
  wire  RAM_ST_clk; // @[ShiftT.scala 39:29]
  wire  RAM_ST_RE; // @[ShiftT.scala 39:29]
  wire [1:0] RAM_ST_RADDR; // @[ShiftT.scala 39:29]
  wire [7:0] RAM_ST_RDATA_0; // @[ShiftT.scala 39:29]
  wire  RAM_ST_WE; // @[ShiftT.scala 39:29]
  wire [1:0] RAM_ST_WADDR; // @[ShiftT.scala 39:29]
  wire [7:0] RAM_ST_WDATA_0; // @[ShiftT.scala 39:29]
  wire  NestedCounters_CE; // @[ShiftT.scala 41:31]
  wire  NestedCounters_valid; // @[ShiftT.scala 41:31]
  reg [1:0] value; // @[Counter.scala 29:33]
  reg [31:0] _RAND_0;
  wire  _T_2 = value == 2'h3; // @[Counter.scala 38:24]
  wire [1:0] _T_4 = value + 2'h1; // @[Counter.scala 39:22]
  RAM_ST RAM_ST ( // @[ShiftT.scala 39:29]
    .clk(RAM_ST_clk),
    .RE(RAM_ST_RE),
    .RADDR(RAM_ST_RADDR),
    .RDATA_0(RAM_ST_RDATA_0),
    .WE(RAM_ST_WE),
    .WADDR(RAM_ST_WADDR),
    .WDATA_0(RAM_ST_WDATA_0)
  );
  NestedCounters_1 NestedCounters ( // @[ShiftT.scala 41:31]
    .CE(NestedCounters_CE),
    .valid(NestedCounters_valid)
  );
  assign valid_down = valid_up; // @[ShiftT.scala 55:14]
  assign O_0 = RAM_ST_RDATA_0; // @[ShiftT.scala 51:7]
  assign RAM_ST_clk = clk;
  assign RAM_ST_RE = valid_up; // @[ShiftT.scala 49:20]
  assign RAM_ST_RADDR = _T_2 ? 2'h0 : _T_4; // @[ShiftT.scala 46:76 ShiftT.scala 47:38]
  assign RAM_ST_WE = valid_up; // @[ShiftT.scala 48:20]
  assign RAM_ST_WADDR = value; // @[ShiftT.scala 45:23]
  assign RAM_ST_WDATA_0 = I_0; // @[ShiftT.scala 50:23]
  assign NestedCounters_CE = valid_up; // @[ShiftT.scala 42:22]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  value = _RAND_0[1:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if (reset) begin
      value <= 2'h0;
    end else if (valid_up) begin
      value <= _T_4;
    end
  end
endmodule
module ShiftT_2(
  input        clk,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  output [7:0] O_0
);
  reg [7:0] _T_0; // @[ShiftT.scala 24:82]
  reg [31:0] _RAND_0;
  assign valid_down = valid_up; // @[ShiftT.scala 55:14]
  assign O_0 = _T_0; // @[ShiftT.scala 24:7]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T_0 = _RAND_0[7:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    _T_0 <= I_0;
  end
endmodule
module SSeqTupleCreator(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0,
  input  [7:0] I1,
  output [7:0] O_0,
  output [7:0] O_1
);
  assign valid_down = valid_up; // @[Tuple.scala 15:14]
  assign O_0 = I0; // @[Tuple.scala 12:32]
  assign O_1 = I1; // @[Tuple.scala 13:32]
endmodule
module Map2S(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  input  [7:0] I1_0,
  output [7:0] O_0_0,
  output [7:0] O_0_1
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1; // @[Map2S.scala 9:22]
  SSeqTupleCreator fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0(fst_op_I0),
    .I1(fst_op_I1),
    .O_0(fst_op_O_0),
    .O_1(fst_op_O_1)
  );
  assign valid_down = fst_op_valid_down; // @[Map2S.scala 26:14]
  assign O_0_0 = fst_op_O_0; // @[Map2S.scala 19:8]
  assign O_0_1 = fst_op_O_1; // @[Map2S.scala 19:8]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0 = I0_0; // @[Map2S.scala 17:13]
  assign fst_op_I1 = I1_0; // @[Map2S.scala 18:13]
endmodule
module Map2T(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  input  [7:0] I1_0,
  output [7:0] O_0_0,
  output [7:0] O_0_1
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1; // @[Map2T.scala 8:20]
  Map2S op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0(op_I0_0),
    .I1_0(op_I1_0),
    .O_0_0(op_O_0_0),
    .O_0_1(op_O_0_1)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_0 = op_O_0_0; // @[Map2T.scala 17:7]
  assign O_0_1 = op_O_0_1; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0 = I0_0; // @[Map2T.scala 15:11]
  assign op_I1_0 = I1_0; // @[Map2T.scala 16:11]
endmodule
module SSeqTupleAppender(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  input  [7:0] I0_1,
  input  [7:0] I1,
  output [7:0] O_0,
  output [7:0] O_1,
  output [7:0] O_2
);
  assign valid_down = valid_up; // @[Tuple.scala 28:14]
  assign O_0 = I0_0; // @[Tuple.scala 24:34]
  assign O_1 = I0_1; // @[Tuple.scala 24:34]
  assign O_2 = I1; // @[Tuple.scala 26:32]
endmodule
module Map2S_1(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I1_0,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2; // @[Map2S.scala 9:22]
  SSeqTupleAppender fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0_0(fst_op_I0_0),
    .I0_1(fst_op_I0_1),
    .I1(fst_op_I1),
    .O_0(fst_op_O_0),
    .O_1(fst_op_O_1),
    .O_2(fst_op_O_2)
  );
  assign valid_down = fst_op_valid_down; // @[Map2S.scala 26:14]
  assign O_0_0 = fst_op_O_0; // @[Map2S.scala 19:8]
  assign O_0_1 = fst_op_O_1; // @[Map2S.scala 19:8]
  assign O_0_2 = fst_op_O_2; // @[Map2S.scala 19:8]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0_0 = I0_0_0; // @[Map2S.scala 17:13]
  assign fst_op_I0_1 = I0_0_1; // @[Map2S.scala 17:13]
  assign fst_op_I1 = I1_0; // @[Map2S.scala 18:13]
endmodule
module Map2T_1(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I1_0,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2; // @[Map2T.scala 8:20]
  Map2S_1 op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0_0(op_I0_0_0),
    .I0_0_1(op_I0_0_1),
    .I1_0(op_I1_0),
    .O_0_0(op_O_0_0),
    .O_0_1(op_O_0_1),
    .O_0_2(op_O_0_2)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_0 = op_O_0_0; // @[Map2T.scala 17:7]
  assign O_0_1 = op_O_0_1; // @[Map2T.scala 17:7]
  assign O_0_2 = op_O_0_2; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0_0 = I0_0_0; // @[Map2T.scala 15:11]
  assign op_I0_0_1 = I0_0_1; // @[Map2T.scala 15:11]
  assign op_I1_0 = I1_0; // @[Map2T.scala 16:11]
endmodule
module PartitionS(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2
);
  assign valid_down = valid_up; // @[Partition.scala 18:14]
  assign O_0_0_0 = I_0_0; // @[Partition.scala 15:39]
  assign O_0_0_1 = I_0_1; // @[Partition.scala 15:39]
  assign O_0_0_2 = I_0_2; // @[Partition.scala 15:39]
endmodule
module MapT(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2
);
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0_2; // @[MapT.scala 8:20]
  PartitionS op ( // @[MapT.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0(op_I_0_0),
    .I_0_1(op_I_0_1),
    .I_0_2(op_I_0_2),
    .O_0_0_0(op_O_0_0_0),
    .O_0_0_1(op_O_0_0_1),
    .O_0_0_2(op_O_0_0_2)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0_0 = op_O_0_0_0; // @[MapT.scala 15:7]
  assign O_0_0_1 = op_O_0_0_1; // @[MapT.scala 15:7]
  assign O_0_0_2 = op_O_0_0_2; // @[MapT.scala 15:7]
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0 = I_0_0; // @[MapT.scala 14:10]
  assign op_I_0_1 = I_0_1; // @[MapT.scala 14:10]
  assign op_I_0_2 = I_0_2; // @[MapT.scala 14:10]
endmodule
module SSeqTupleToSSeq(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  input  [7:0] I_1,
  input  [7:0] I_2,
  output [7:0] O_0,
  output [7:0] O_1,
  output [7:0] O_2
);
  assign valid_down = valid_up; // @[Tuple.scala 42:14]
  assign O_0 = I_0; // @[Tuple.scala 41:5]
  assign O_1 = I_1; // @[Tuple.scala 41:5]
  assign O_2 = I_2; // @[Tuple.scala 41:5]
endmodule
module Remove1S(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  output [7:0] O_0,
  output [7:0] O_1,
  output [7:0] O_2
);
  wire  op_inst_valid_up; // @[Remove1S.scala 9:23]
  wire  op_inst_valid_down; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_I_0; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_I_1; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_I_2; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_O_0; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_O_1; // @[Remove1S.scala 9:23]
  wire [7:0] op_inst_O_2; // @[Remove1S.scala 9:23]
  SSeqTupleToSSeq op_inst ( // @[Remove1S.scala 9:23]
    .valid_up(op_inst_valid_up),
    .valid_down(op_inst_valid_down),
    .I_0(op_inst_I_0),
    .I_1(op_inst_I_1),
    .I_2(op_inst_I_2),
    .O_0(op_inst_O_0),
    .O_1(op_inst_O_1),
    .O_2(op_inst_O_2)
  );
  assign valid_down = op_inst_valid_down; // @[Remove1S.scala 16:14]
  assign O_0 = op_inst_O_0; // @[Remove1S.scala 14:5]
  assign O_1 = op_inst_O_1; // @[Remove1S.scala 14:5]
  assign O_2 = op_inst_O_2; // @[Remove1S.scala 14:5]
  assign op_inst_valid_up = valid_up; // @[Remove1S.scala 15:20]
  assign op_inst_I_0 = I_0_0; // @[Remove1S.scala 13:13]
  assign op_inst_I_1 = I_0_1; // @[Remove1S.scala 13:13]
  assign op_inst_I_2 = I_0_2; // @[Remove1S.scala 13:13]
endmodule
module MapS(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_0,
  input  [7:0] I_0_0_1,
  input  [7:0] I_0_0_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2
);
  wire  fst_op_valid_up; // @[MapS.scala 9:22]
  wire  fst_op_valid_down; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0_0; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0_1; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0_2; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_0; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_1; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_2; // @[MapS.scala 9:22]
  Remove1S fst_op ( // @[MapS.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I_0_0(fst_op_I_0_0),
    .I_0_1(fst_op_I_0_1),
    .I_0_2(fst_op_I_0_2),
    .O_0(fst_op_O_0),
    .O_1(fst_op_O_1),
    .O_2(fst_op_O_2)
  );
  assign valid_down = fst_op_valid_down; // @[MapS.scala 23:14]
  assign O_0_0 = fst_op_O_0; // @[MapS.scala 17:8]
  assign O_0_1 = fst_op_O_1; // @[MapS.scala 17:8]
  assign O_0_2 = fst_op_O_2; // @[MapS.scala 17:8]
  assign fst_op_valid_up = valid_up; // @[MapS.scala 15:19]
  assign fst_op_I_0_0 = I_0_0_0; // @[MapS.scala 16:12]
  assign fst_op_I_0_1 = I_0_0_1; // @[MapS.scala 16:12]
  assign fst_op_I_0_2 = I_0_0_2; // @[MapS.scala 16:12]
endmodule
module MapT_1(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_0,
  input  [7:0] I_0_0_1,
  input  [7:0] I_0_0_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2
);
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_2; // @[MapT.scala 8:20]
  MapS op ( // @[MapT.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0_0(op_I_0_0_0),
    .I_0_0_1(op_I_0_0_1),
    .I_0_0_2(op_I_0_0_2),
    .O_0_0(op_O_0_0),
    .O_0_1(op_O_0_1),
    .O_0_2(op_O_0_2)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0 = op_O_0_0; // @[MapT.scala 15:7]
  assign O_0_1 = op_O_0_1; // @[MapT.scala 15:7]
  assign O_0_2 = op_O_0_2; // @[MapT.scala 15:7]
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0_0 = I_0_0_0; // @[MapT.scala 14:10]
  assign op_I_0_0_1 = I_0_0_1; // @[MapT.scala 14:10]
  assign op_I_0_0_2 = I_0_0_2; // @[MapT.scala 14:10]
endmodule
module SSeqTupleCreator_2(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  input  [7:0] I0_1,
  input  [7:0] I0_2,
  input  [7:0] I1_0,
  input  [7:0] I1_1,
  input  [7:0] I1_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2
);
  assign valid_down = valid_up; // @[Tuple.scala 15:14]
  assign O_0_0 = I0_0; // @[Tuple.scala 12:32]
  assign O_0_1 = I0_1; // @[Tuple.scala 12:32]
  assign O_0_2 = I0_2; // @[Tuple.scala 12:32]
  assign O_1_0 = I1_0; // @[Tuple.scala 13:32]
  assign O_1_1 = I1_1; // @[Tuple.scala 13:32]
  assign O_1_2 = I1_2; // @[Tuple.scala 13:32]
endmodule
module Map2S_4(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I0_0_2,
  input  [7:0] I1_0_0,
  input  [7:0] I1_0_1,
  input  [7:0] I1_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2,
  output [7:0] O_0_1_0,
  output [7:0] O_0_1_1,
  output [7:0] O_0_1_2
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_2; // @[Map2S.scala 9:22]
  SSeqTupleCreator_2 fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0_0(fst_op_I0_0),
    .I0_1(fst_op_I0_1),
    .I0_2(fst_op_I0_2),
    .I1_0(fst_op_I1_0),
    .I1_1(fst_op_I1_1),
    .I1_2(fst_op_I1_2),
    .O_0_0(fst_op_O_0_0),
    .O_0_1(fst_op_O_0_1),
    .O_0_2(fst_op_O_0_2),
    .O_1_0(fst_op_O_1_0),
    .O_1_1(fst_op_O_1_1),
    .O_1_2(fst_op_O_1_2)
  );
  assign valid_down = fst_op_valid_down; // @[Map2S.scala 26:14]
  assign O_0_0_0 = fst_op_O_0_0; // @[Map2S.scala 19:8]
  assign O_0_0_1 = fst_op_O_0_1; // @[Map2S.scala 19:8]
  assign O_0_0_2 = fst_op_O_0_2; // @[Map2S.scala 19:8]
  assign O_0_1_0 = fst_op_O_1_0; // @[Map2S.scala 19:8]
  assign O_0_1_1 = fst_op_O_1_1; // @[Map2S.scala 19:8]
  assign O_0_1_2 = fst_op_O_1_2; // @[Map2S.scala 19:8]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0_0 = I0_0_0; // @[Map2S.scala 17:13]
  assign fst_op_I0_1 = I0_0_1; // @[Map2S.scala 17:13]
  assign fst_op_I0_2 = I0_0_2; // @[Map2S.scala 17:13]
  assign fst_op_I1_0 = I1_0_0; // @[Map2S.scala 18:13]
  assign fst_op_I1_1 = I1_0_1; // @[Map2S.scala 18:13]
  assign fst_op_I1_2 = I1_0_2; // @[Map2S.scala 18:13]
endmodule
module Map2T_4(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I0_0_2,
  input  [7:0] I1_0_0,
  input  [7:0] I1_0_1,
  input  [7:0] I1_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2,
  output [7:0] O_0_1_0,
  output [7:0] O_0_1_1,
  output [7:0] O_0_1_2
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_2; // @[Map2T.scala 8:20]
  Map2S_4 op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0_0(op_I0_0_0),
    .I0_0_1(op_I0_0_1),
    .I0_0_2(op_I0_0_2),
    .I1_0_0(op_I1_0_0),
    .I1_0_1(op_I1_0_1),
    .I1_0_2(op_I1_0_2),
    .O_0_0_0(op_O_0_0_0),
    .O_0_0_1(op_O_0_0_1),
    .O_0_0_2(op_O_0_0_2),
    .O_0_1_0(op_O_0_1_0),
    .O_0_1_1(op_O_0_1_1),
    .O_0_1_2(op_O_0_1_2)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_0_0 = op_O_0_0_0; // @[Map2T.scala 17:7]
  assign O_0_0_1 = op_O_0_0_1; // @[Map2T.scala 17:7]
  assign O_0_0_2 = op_O_0_0_2; // @[Map2T.scala 17:7]
  assign O_0_1_0 = op_O_0_1_0; // @[Map2T.scala 17:7]
  assign O_0_1_1 = op_O_0_1_1; // @[Map2T.scala 17:7]
  assign O_0_1_2 = op_O_0_1_2; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0_0 = I0_0_0; // @[Map2T.scala 15:11]
  assign op_I0_0_1 = I0_0_1; // @[Map2T.scala 15:11]
  assign op_I0_0_2 = I0_0_2; // @[Map2T.scala 15:11]
  assign op_I1_0_0 = I1_0_0; // @[Map2T.scala 16:11]
  assign op_I1_0_1 = I1_0_1; // @[Map2T.scala 16:11]
  assign op_I1_0_2 = I1_0_2; // @[Map2T.scala 16:11]
endmodule
module SSeqTupleAppender_3(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I0_0_2,
  input  [7:0] I0_1_0,
  input  [7:0] I0_1_1,
  input  [7:0] I0_1_2,
  input  [7:0] I1_0,
  input  [7:0] I1_1,
  input  [7:0] I1_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2,
  output [7:0] O_2_0,
  output [7:0] O_2_1,
  output [7:0] O_2_2
);
  assign valid_down = valid_up; // @[Tuple.scala 28:14]
  assign O_0_0 = I0_0_0; // @[Tuple.scala 24:34]
  assign O_0_1 = I0_0_1; // @[Tuple.scala 24:34]
  assign O_0_2 = I0_0_2; // @[Tuple.scala 24:34]
  assign O_1_0 = I0_1_0; // @[Tuple.scala 24:34]
  assign O_1_1 = I0_1_1; // @[Tuple.scala 24:34]
  assign O_1_2 = I0_1_2; // @[Tuple.scala 24:34]
  assign O_2_0 = I1_0; // @[Tuple.scala 26:32]
  assign O_2_1 = I1_1; // @[Tuple.scala 26:32]
  assign O_2_2 = I1_2; // @[Tuple.scala 26:32]
endmodule
module Map2S_7(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0_0,
  input  [7:0] I0_0_0_1,
  input  [7:0] I0_0_0_2,
  input  [7:0] I0_0_1_0,
  input  [7:0] I0_0_1_1,
  input  [7:0] I0_0_1_2,
  input  [7:0] I1_0_0,
  input  [7:0] I1_0_1,
  input  [7:0] I1_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2,
  output [7:0] O_0_1_0,
  output [7:0] O_0_1_1,
  output [7:0] O_0_1_2,
  output [7:0] O_0_2_0,
  output [7:0] O_0_2_1,
  output [7:0] O_0_2_2
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2_2; // @[Map2S.scala 9:22]
  SSeqTupleAppender_3 fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0_0_0(fst_op_I0_0_0),
    .I0_0_1(fst_op_I0_0_1),
    .I0_0_2(fst_op_I0_0_2),
    .I0_1_0(fst_op_I0_1_0),
    .I0_1_1(fst_op_I0_1_1),
    .I0_1_2(fst_op_I0_1_2),
    .I1_0(fst_op_I1_0),
    .I1_1(fst_op_I1_1),
    .I1_2(fst_op_I1_2),
    .O_0_0(fst_op_O_0_0),
    .O_0_1(fst_op_O_0_1),
    .O_0_2(fst_op_O_0_2),
    .O_1_0(fst_op_O_1_0),
    .O_1_1(fst_op_O_1_1),
    .O_1_2(fst_op_O_1_2),
    .O_2_0(fst_op_O_2_0),
    .O_2_1(fst_op_O_2_1),
    .O_2_2(fst_op_O_2_2)
  );
  assign valid_down = fst_op_valid_down; // @[Map2S.scala 26:14]
  assign O_0_0_0 = fst_op_O_0_0; // @[Map2S.scala 19:8]
  assign O_0_0_1 = fst_op_O_0_1; // @[Map2S.scala 19:8]
  assign O_0_0_2 = fst_op_O_0_2; // @[Map2S.scala 19:8]
  assign O_0_1_0 = fst_op_O_1_0; // @[Map2S.scala 19:8]
  assign O_0_1_1 = fst_op_O_1_1; // @[Map2S.scala 19:8]
  assign O_0_1_2 = fst_op_O_1_2; // @[Map2S.scala 19:8]
  assign O_0_2_0 = fst_op_O_2_0; // @[Map2S.scala 19:8]
  assign O_0_2_1 = fst_op_O_2_1; // @[Map2S.scala 19:8]
  assign O_0_2_2 = fst_op_O_2_2; // @[Map2S.scala 19:8]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0_0_0 = I0_0_0_0; // @[Map2S.scala 17:13]
  assign fst_op_I0_0_1 = I0_0_0_1; // @[Map2S.scala 17:13]
  assign fst_op_I0_0_2 = I0_0_0_2; // @[Map2S.scala 17:13]
  assign fst_op_I0_1_0 = I0_0_1_0; // @[Map2S.scala 17:13]
  assign fst_op_I0_1_1 = I0_0_1_1; // @[Map2S.scala 17:13]
  assign fst_op_I0_1_2 = I0_0_1_2; // @[Map2S.scala 17:13]
  assign fst_op_I1_0 = I1_0_0; // @[Map2S.scala 18:13]
  assign fst_op_I1_1 = I1_0_1; // @[Map2S.scala 18:13]
  assign fst_op_I1_2 = I1_0_2; // @[Map2S.scala 18:13]
endmodule
module Map2T_7(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0_0,
  input  [7:0] I0_0_0_1,
  input  [7:0] I0_0_0_2,
  input  [7:0] I0_0_1_0,
  input  [7:0] I0_0_1_1,
  input  [7:0] I0_0_1_2,
  input  [7:0] I1_0_0,
  input  [7:0] I1_0_1,
  input  [7:0] I1_0_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2,
  output [7:0] O_0_1_0,
  output [7:0] O_0_1_1,
  output [7:0] O_0_1_2,
  output [7:0] O_0_2_0,
  output [7:0] O_0_2_1,
  output [7:0] O_0_2_2
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1_2; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I1_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2_1; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2_2; // @[Map2T.scala 8:20]
  Map2S_7 op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0_0_0(op_I0_0_0_0),
    .I0_0_0_1(op_I0_0_0_1),
    .I0_0_0_2(op_I0_0_0_2),
    .I0_0_1_0(op_I0_0_1_0),
    .I0_0_1_1(op_I0_0_1_1),
    .I0_0_1_2(op_I0_0_1_2),
    .I1_0_0(op_I1_0_0),
    .I1_0_1(op_I1_0_1),
    .I1_0_2(op_I1_0_2),
    .O_0_0_0(op_O_0_0_0),
    .O_0_0_1(op_O_0_0_1),
    .O_0_0_2(op_O_0_0_2),
    .O_0_1_0(op_O_0_1_0),
    .O_0_1_1(op_O_0_1_1),
    .O_0_1_2(op_O_0_1_2),
    .O_0_2_0(op_O_0_2_0),
    .O_0_2_1(op_O_0_2_1),
    .O_0_2_2(op_O_0_2_2)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_0_0 = op_O_0_0_0; // @[Map2T.scala 17:7]
  assign O_0_0_1 = op_O_0_0_1; // @[Map2T.scala 17:7]
  assign O_0_0_2 = op_O_0_0_2; // @[Map2T.scala 17:7]
  assign O_0_1_0 = op_O_0_1_0; // @[Map2T.scala 17:7]
  assign O_0_1_1 = op_O_0_1_1; // @[Map2T.scala 17:7]
  assign O_0_1_2 = op_O_0_1_2; // @[Map2T.scala 17:7]
  assign O_0_2_0 = op_O_0_2_0; // @[Map2T.scala 17:7]
  assign O_0_2_1 = op_O_0_2_1; // @[Map2T.scala 17:7]
  assign O_0_2_2 = op_O_0_2_2; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0_0_0 = I0_0_0_0; // @[Map2T.scala 15:11]
  assign op_I0_0_0_1 = I0_0_0_1; // @[Map2T.scala 15:11]
  assign op_I0_0_0_2 = I0_0_0_2; // @[Map2T.scala 15:11]
  assign op_I0_0_1_0 = I0_0_1_0; // @[Map2T.scala 15:11]
  assign op_I0_0_1_1 = I0_0_1_1; // @[Map2T.scala 15:11]
  assign op_I0_0_1_2 = I0_0_1_2; // @[Map2T.scala 15:11]
  assign op_I1_0_0 = I1_0_0; // @[Map2T.scala 16:11]
  assign op_I1_0_1 = I1_0_1; // @[Map2T.scala 16:11]
  assign op_I1_0_2 = I1_0_2; // @[Map2T.scala 16:11]
endmodule
module Passthrough(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_0,
  input  [7:0] I_0_0_1,
  input  [7:0] I_0_0_2,
  input  [7:0] I_0_1_0,
  input  [7:0] I_0_1_1,
  input  [7:0] I_0_1_2,
  input  [7:0] I_0_2_0,
  input  [7:0] I_0_2_1,
  input  [7:0] I_0_2_2,
  output [7:0] O_0_0_0,
  output [7:0] O_0_0_1,
  output [7:0] O_0_0_2,
  output [7:0] O_0_1_0,
  output [7:0] O_0_1_1,
  output [7:0] O_0_1_2,
  output [7:0] O_0_2_0,
  output [7:0] O_0_2_1,
  output [7:0] O_0_2_2
);
  assign valid_down = valid_up; // @[Passthrough.scala 18:14]
  assign O_0_0_0 = I_0_0_0; // @[Passthrough.scala 17:68]
  assign O_0_0_1 = I_0_0_1; // @[Passthrough.scala 17:68]
  assign O_0_0_2 = I_0_0_2; // @[Passthrough.scala 17:68]
  assign O_0_1_0 = I_0_1_0; // @[Passthrough.scala 17:68]
  assign O_0_1_1 = I_0_1_1; // @[Passthrough.scala 17:68]
  assign O_0_1_2 = I_0_1_2; // @[Passthrough.scala 17:68]
  assign O_0_2_0 = I_0_2_0; // @[Passthrough.scala 17:68]
  assign O_0_2_1 = I_0_2_1; // @[Passthrough.scala 17:68]
  assign O_0_2_2 = I_0_2_2; // @[Passthrough.scala 17:68]
endmodule
module Passthrough_1(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_0,
  input  [7:0] I_0_0_1,
  input  [7:0] I_0_0_2,
  input  [7:0] I_0_1_0,
  input  [7:0] I_0_1_1,
  input  [7:0] I_0_1_2,
  input  [7:0] I_0_2_0,
  input  [7:0] I_0_2_1,
  input  [7:0] I_0_2_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2,
  output [7:0] O_2_0,
  output [7:0] O_2_1,
  output [7:0] O_2_2
);
  assign valid_down = valid_up; // @[Passthrough.scala 18:14]
  assign O_0_0 = I_0_0_0; // @[Passthrough.scala 17:68]
  assign O_0_1 = I_0_0_1; // @[Passthrough.scala 17:68]
  assign O_0_2 = I_0_0_2; // @[Passthrough.scala 17:68]
  assign O_1_0 = I_0_1_0; // @[Passthrough.scala 17:68]
  assign O_1_1 = I_0_1_1; // @[Passthrough.scala 17:68]
  assign O_1_2 = I_0_1_2; // @[Passthrough.scala 17:68]
  assign O_2_0 = I_0_2_0; // @[Passthrough.scala 17:68]
  assign O_2_1 = I_0_2_1; // @[Passthrough.scala 17:68]
  assign O_2_2 = I_0_2_2; // @[Passthrough.scala 17:68]
endmodule
module MapT_6(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_0,
  input  [7:0] I_0_0_1,
  input  [7:0] I_0_0_2,
  input  [7:0] I_0_1_0,
  input  [7:0] I_0_1_1,
  input  [7:0] I_0_1_2,
  input  [7:0] I_0_2_0,
  input  [7:0] I_0_2_1,
  input  [7:0] I_0_2_2,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2,
  output [7:0] O_2_0,
  output [7:0] O_2_1,
  output [7:0] O_2_2
);
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_2; // @[MapT.scala 8:20]
  Passthrough_1 op ( // @[MapT.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0_0(op_I_0_0_0),
    .I_0_0_1(op_I_0_0_1),
    .I_0_0_2(op_I_0_0_2),
    .I_0_1_0(op_I_0_1_0),
    .I_0_1_1(op_I_0_1_1),
    .I_0_1_2(op_I_0_1_2),
    .I_0_2_0(op_I_0_2_0),
    .I_0_2_1(op_I_0_2_1),
    .I_0_2_2(op_I_0_2_2),
    .O_0_0(op_O_0_0),
    .O_0_1(op_O_0_1),
    .O_0_2(op_O_0_2),
    .O_1_0(op_O_1_0),
    .O_1_1(op_O_1_1),
    .O_1_2(op_O_1_2),
    .O_2_0(op_O_2_0),
    .O_2_1(op_O_2_1),
    .O_2_2(op_O_2_2)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0 = op_O_0_0; // @[MapT.scala 15:7]
  assign O_0_1 = op_O_0_1; // @[MapT.scala 15:7]
  assign O_0_2 = op_O_0_2; // @[MapT.scala 15:7]
  assign O_1_0 = op_O_1_0; // @[MapT.scala 15:7]
  assign O_1_1 = op_O_1_1; // @[MapT.scala 15:7]
  assign O_1_2 = op_O_1_2; // @[MapT.scala 15:7]
  assign O_2_0 = op_O_2_0; // @[MapT.scala 15:7]
  assign O_2_1 = op_O_2_1; // @[MapT.scala 15:7]
  assign O_2_2 = op_O_2_2; // @[MapT.scala 15:7]
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0_0 = I_0_0_0; // @[MapT.scala 14:10]
  assign op_I_0_0_1 = I_0_0_1; // @[MapT.scala 14:10]
  assign op_I_0_0_2 = I_0_0_2; // @[MapT.scala 14:10]
  assign op_I_0_1_0 = I_0_1_0; // @[MapT.scala 14:10]
  assign op_I_0_1_1 = I_0_1_1; // @[MapT.scala 14:10]
  assign op_I_0_1_2 = I_0_1_2; // @[MapT.scala 14:10]
  assign op_I_0_2_0 = I_0_2_0; // @[MapT.scala 14:10]
  assign op_I_0_2_1 = I_0_2_1; // @[MapT.scala 14:10]
  assign op_I_0_2_2 = I_0_2_2; // @[MapT.scala 14:10]
endmodule
module InitialDelayCounter(
  input   clk,
  input   reset,
  output  valid_down
);
  reg  value; // @[InitialDelayCounter.scala 8:34]
  reg [31:0] _RAND_0;
  wire  _T_1 = value < 1'h1; // @[InitialDelayCounter.scala 17:17]
  wire  _T_4 = value + 1'h1; // @[InitialDelayCounter.scala 17:53]
  assign valid_down = value; // @[InitialDelayCounter.scala 16:16]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  value = _RAND_0[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if (reset) begin
      value <= 1'h0;
    end else if (_T_1) begin
      value <= _T_4;
    end
  end
endmodule
module AtomTuple(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0,
  input  [7:0] I1,
  output [7:0] O_t0b,
  output [7:0] O_t1b
);
  assign valid_down = valid_up; // @[Tuple.scala 51:14]
  assign O_t0b = I0; // @[Tuple.scala 49:9]
  assign O_t1b = I1; // @[Tuple.scala 50:9]
endmodule
module Map2S_8(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  input  [7:0] I0_1,
  input  [7:0] I0_2,
  input  [7:0] I1_0,
  input  [7:0] I1_1,
  input  [7:0] I1_2,
  output [7:0] O_0_t0b,
  output [7:0] O_0_t1b,
  output [7:0] O_1_t0b,
  output [7:0] O_1_t1b,
  output [7:0] O_2_t0b,
  output [7:0] O_2_t1b
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_t0b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_t1b; // @[Map2S.scala 9:22]
  wire  other_ops_0_valid_up; // @[Map2S.scala 10:86]
  wire  other_ops_0_valid_down; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_t1b; // @[Map2S.scala 10:86]
  wire  other_ops_1_valid_up; // @[Map2S.scala 10:86]
  wire  other_ops_1_valid_down; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_t1b; // @[Map2S.scala 10:86]
  wire  _T = fst_op_valid_down & other_ops_0_valid_down; // @[Map2S.scala 26:83]
  AtomTuple fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0(fst_op_I0),
    .I1(fst_op_I1),
    .O_t0b(fst_op_O_t0b),
    .O_t1b(fst_op_O_t1b)
  );
  AtomTuple other_ops_0 ( // @[Map2S.scala 10:86]
    .valid_up(other_ops_0_valid_up),
    .valid_down(other_ops_0_valid_down),
    .I0(other_ops_0_I0),
    .I1(other_ops_0_I1),
    .O_t0b(other_ops_0_O_t0b),
    .O_t1b(other_ops_0_O_t1b)
  );
  AtomTuple other_ops_1 ( // @[Map2S.scala 10:86]
    .valid_up(other_ops_1_valid_up),
    .valid_down(other_ops_1_valid_down),
    .I0(other_ops_1_I0),
    .I1(other_ops_1_I1),
    .O_t0b(other_ops_1_O_t0b),
    .O_t1b(other_ops_1_O_t1b)
  );
  assign valid_down = _T & other_ops_1_valid_down; // @[Map2S.scala 26:14]
  assign O_0_t0b = fst_op_O_t0b; // @[Map2S.scala 19:8]
  assign O_0_t1b = fst_op_O_t1b; // @[Map2S.scala 19:8]
  assign O_1_t0b = other_ops_0_O_t0b; // @[Map2S.scala 24:12]
  assign O_1_t1b = other_ops_0_O_t1b; // @[Map2S.scala 24:12]
  assign O_2_t0b = other_ops_1_O_t0b; // @[Map2S.scala 24:12]
  assign O_2_t1b = other_ops_1_O_t1b; // @[Map2S.scala 24:12]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0 = I0_0; // @[Map2S.scala 17:13]
  assign fst_op_I1 = I1_0; // @[Map2S.scala 18:13]
  assign other_ops_0_valid_up = valid_up; // @[Map2S.scala 21:39]
  assign other_ops_0_I0 = I0_1; // @[Map2S.scala 22:43]
  assign other_ops_0_I1 = I1_1; // @[Map2S.scala 23:43]
  assign other_ops_1_valid_up = valid_up; // @[Map2S.scala 21:39]
  assign other_ops_1_I0 = I0_2; // @[Map2S.scala 22:43]
  assign other_ops_1_I1 = I1_2; // @[Map2S.scala 23:43]
endmodule
module Map2S_9(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I0_0_2,
  input  [7:0] I0_1_0,
  input  [7:0] I0_1_1,
  input  [7:0] I0_1_2,
  input  [7:0] I0_2_0,
  input  [7:0] I0_2_1,
  input  [7:0] I0_2_2,
  output [7:0] O_0_0_t0b,
  output [7:0] O_0_0_t1b,
  output [7:0] O_0_1_t0b,
  output [7:0] O_0_1_t1b,
  output [7:0] O_0_2_t0b,
  output [7:0] O_0_2_t1b,
  output [7:0] O_1_0_t0b,
  output [7:0] O_1_0_t1b,
  output [7:0] O_1_1_t0b,
  output [7:0] O_1_1_t1b,
  output [7:0] O_1_2_t0b,
  output [7:0] O_1_2_t1b,
  output [7:0] O_2_0_t0b,
  output [7:0] O_2_0_t1b,
  output [7:0] O_2_1_t0b,
  output [7:0] O_2_1_t1b,
  output [7:0] O_2_2_t0b,
  output [7:0] O_2_2_t1b
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_1; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I1_2; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_t0b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_0_t1b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_t0b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_1_t1b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2_t0b; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_2_t1b; // @[Map2S.scala 9:22]
  wire  other_ops_0_valid_up; // @[Map2S.scala 10:86]
  wire  other_ops_0_valid_down; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I0_0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I0_1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I0_2; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I1_0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I1_1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_I1_2; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_0_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_0_t1b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_1_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_1_t1b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_2_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_0_O_2_t1b; // @[Map2S.scala 10:86]
  wire  other_ops_1_valid_up; // @[Map2S.scala 10:86]
  wire  other_ops_1_valid_down; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I0_0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I0_1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I0_2; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I1_0; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I1_1; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_I1_2; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_0_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_0_t1b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_1_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_1_t1b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_2_t0b; // @[Map2S.scala 10:86]
  wire [7:0] other_ops_1_O_2_t1b; // @[Map2S.scala 10:86]
  wire  _T = fst_op_valid_down & other_ops_0_valid_down; // @[Map2S.scala 26:83]
  Map2S_8 fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0_0(fst_op_I0_0),
    .I0_1(fst_op_I0_1),
    .I0_2(fst_op_I0_2),
    .I1_0(fst_op_I1_0),
    .I1_1(fst_op_I1_1),
    .I1_2(fst_op_I1_2),
    .O_0_t0b(fst_op_O_0_t0b),
    .O_0_t1b(fst_op_O_0_t1b),
    .O_1_t0b(fst_op_O_1_t0b),
    .O_1_t1b(fst_op_O_1_t1b),
    .O_2_t0b(fst_op_O_2_t0b),
    .O_2_t1b(fst_op_O_2_t1b)
  );
  Map2S_8 other_ops_0 ( // @[Map2S.scala 10:86]
    .valid_up(other_ops_0_valid_up),
    .valid_down(other_ops_0_valid_down),
    .I0_0(other_ops_0_I0_0),
    .I0_1(other_ops_0_I0_1),
    .I0_2(other_ops_0_I0_2),
    .I1_0(other_ops_0_I1_0),
    .I1_1(other_ops_0_I1_1),
    .I1_2(other_ops_0_I1_2),
    .O_0_t0b(other_ops_0_O_0_t0b),
    .O_0_t1b(other_ops_0_O_0_t1b),
    .O_1_t0b(other_ops_0_O_1_t0b),
    .O_1_t1b(other_ops_0_O_1_t1b),
    .O_2_t0b(other_ops_0_O_2_t0b),
    .O_2_t1b(other_ops_0_O_2_t1b)
  );
  Map2S_8 other_ops_1 ( // @[Map2S.scala 10:86]
    .valid_up(other_ops_1_valid_up),
    .valid_down(other_ops_1_valid_down),
    .I0_0(other_ops_1_I0_0),
    .I0_1(other_ops_1_I0_1),
    .I0_2(other_ops_1_I0_2),
    .I1_0(other_ops_1_I1_0),
    .I1_1(other_ops_1_I1_1),
    .I1_2(other_ops_1_I1_2),
    .O_0_t0b(other_ops_1_O_0_t0b),
    .O_0_t1b(other_ops_1_O_0_t1b),
    .O_1_t0b(other_ops_1_O_1_t0b),
    .O_1_t1b(other_ops_1_O_1_t1b),
    .O_2_t0b(other_ops_1_O_2_t0b),
    .O_2_t1b(other_ops_1_O_2_t1b)
  );
  assign valid_down = _T & other_ops_1_valid_down; // @[Map2S.scala 26:14]
  assign O_0_0_t0b = fst_op_O_0_t0b; // @[Map2S.scala 19:8]
  assign O_0_0_t1b = fst_op_O_0_t1b; // @[Map2S.scala 19:8]
  assign O_0_1_t0b = fst_op_O_1_t0b; // @[Map2S.scala 19:8]
  assign O_0_1_t1b = fst_op_O_1_t1b; // @[Map2S.scala 19:8]
  assign O_0_2_t0b = fst_op_O_2_t0b; // @[Map2S.scala 19:8]
  assign O_0_2_t1b = fst_op_O_2_t1b; // @[Map2S.scala 19:8]
  assign O_1_0_t0b = other_ops_0_O_0_t0b; // @[Map2S.scala 24:12]
  assign O_1_0_t1b = other_ops_0_O_0_t1b; // @[Map2S.scala 24:12]
  assign O_1_1_t0b = other_ops_0_O_1_t0b; // @[Map2S.scala 24:12]
  assign O_1_1_t1b = other_ops_0_O_1_t1b; // @[Map2S.scala 24:12]
  assign O_1_2_t0b = other_ops_0_O_2_t0b; // @[Map2S.scala 24:12]
  assign O_1_2_t1b = other_ops_0_O_2_t1b; // @[Map2S.scala 24:12]
  assign O_2_0_t0b = other_ops_1_O_0_t0b; // @[Map2S.scala 24:12]
  assign O_2_0_t1b = other_ops_1_O_0_t1b; // @[Map2S.scala 24:12]
  assign O_2_1_t0b = other_ops_1_O_1_t0b; // @[Map2S.scala 24:12]
  assign O_2_1_t1b = other_ops_1_O_1_t1b; // @[Map2S.scala 24:12]
  assign O_2_2_t0b = other_ops_1_O_2_t0b; // @[Map2S.scala 24:12]
  assign O_2_2_t1b = other_ops_1_O_2_t1b; // @[Map2S.scala 24:12]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0_0 = I0_0_0; // @[Map2S.scala 17:13]
  assign fst_op_I0_1 = I0_0_1; // @[Map2S.scala 17:13]
  assign fst_op_I0_2 = I0_0_2; // @[Map2S.scala 17:13]
  assign fst_op_I1_0 = 8'h1; // @[Map2S.scala 18:13]
  assign fst_op_I1_1 = 8'h2; // @[Map2S.scala 18:13]
  assign fst_op_I1_2 = 8'h1; // @[Map2S.scala 18:13]
  assign other_ops_0_valid_up = valid_up; // @[Map2S.scala 21:39]
  assign other_ops_0_I0_0 = I0_1_0; // @[Map2S.scala 22:43]
  assign other_ops_0_I0_1 = I0_1_1; // @[Map2S.scala 22:43]
  assign other_ops_0_I0_2 = I0_1_2; // @[Map2S.scala 22:43]
  assign other_ops_0_I1_0 = 8'h2; // @[Map2S.scala 23:43]
  assign other_ops_0_I1_1 = 8'h4; // @[Map2S.scala 23:43]
  assign other_ops_0_I1_2 = 8'h2; // @[Map2S.scala 23:43]
  assign other_ops_1_valid_up = valid_up; // @[Map2S.scala 21:39]
  assign other_ops_1_I0_0 = I0_2_0; // @[Map2S.scala 22:43]
  assign other_ops_1_I0_1 = I0_2_1; // @[Map2S.scala 22:43]
  assign other_ops_1_I0_2 = I0_2_2; // @[Map2S.scala 22:43]
  assign other_ops_1_I1_0 = 8'h1; // @[Map2S.scala 23:43]
  assign other_ops_1_I1_1 = 8'h2; // @[Map2S.scala 23:43]
  assign other_ops_1_I1_2 = 8'h1; // @[Map2S.scala 23:43]
endmodule
module Map2T_8(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0_0,
  input  [7:0] I0_0_1,
  input  [7:0] I0_0_2,
  input  [7:0] I0_1_0,
  input  [7:0] I0_1_1,
  input  [7:0] I0_1_2,
  input  [7:0] I0_2_0,
  input  [7:0] I0_2_1,
  input  [7:0] I0_2_2,
  output [7:0] O_0_0_t0b,
  output [7:0] O_0_0_t1b,
  output [7:0] O_0_1_t0b,
  output [7:0] O_0_1_t1b,
  output [7:0] O_0_2_t0b,
  output [7:0] O_0_2_t1b,
  output [7:0] O_1_0_t0b,
  output [7:0] O_1_0_t1b,
  output [7:0] O_1_1_t0b,
  output [7:0] O_1_1_t1b,
  output [7:0] O_1_2_t0b,
  output [7:0] O_1_2_t1b,
  output [7:0] O_2_0_t0b,
  output [7:0] O_2_0_t1b,
  output [7:0] O_2_1_t0b,
  output [7:0] O_2_1_t1b,
  output [7:0] O_2_2_t0b,
  output [7:0] O_2_2_t1b
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0_2; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_1_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_1_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_1_2; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_2_0; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_2_1; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_2_2; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_0_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_1_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_2_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_0_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_0_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_1_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_1_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_2_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_1_2_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_0_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_0_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_1_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_1_t1b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_2_t0b; // @[Map2T.scala 8:20]
  wire [7:0] op_O_2_2_t1b; // @[Map2T.scala 8:20]
  Map2S_9 op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0_0(op_I0_0_0),
    .I0_0_1(op_I0_0_1),
    .I0_0_2(op_I0_0_2),
    .I0_1_0(op_I0_1_0),
    .I0_1_1(op_I0_1_1),
    .I0_1_2(op_I0_1_2),
    .I0_2_0(op_I0_2_0),
    .I0_2_1(op_I0_2_1),
    .I0_2_2(op_I0_2_2),
    .O_0_0_t0b(op_O_0_0_t0b),
    .O_0_0_t1b(op_O_0_0_t1b),
    .O_0_1_t0b(op_O_0_1_t0b),
    .O_0_1_t1b(op_O_0_1_t1b),
    .O_0_2_t0b(op_O_0_2_t0b),
    .O_0_2_t1b(op_O_0_2_t1b),
    .O_1_0_t0b(op_O_1_0_t0b),
    .O_1_0_t1b(op_O_1_0_t1b),
    .O_1_1_t0b(op_O_1_1_t0b),
    .O_1_1_t1b(op_O_1_1_t1b),
    .O_1_2_t0b(op_O_1_2_t0b),
    .O_1_2_t1b(op_O_1_2_t1b),
    .O_2_0_t0b(op_O_2_0_t0b),
    .O_2_0_t1b(op_O_2_0_t1b),
    .O_2_1_t0b(op_O_2_1_t0b),
    .O_2_1_t1b(op_O_2_1_t1b),
    .O_2_2_t0b(op_O_2_2_t0b),
    .O_2_2_t1b(op_O_2_2_t1b)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_0_t0b = op_O_0_0_t0b; // @[Map2T.scala 17:7]
  assign O_0_0_t1b = op_O_0_0_t1b; // @[Map2T.scala 17:7]
  assign O_0_1_t0b = op_O_0_1_t0b; // @[Map2T.scala 17:7]
  assign O_0_1_t1b = op_O_0_1_t1b; // @[Map2T.scala 17:7]
  assign O_0_2_t0b = op_O_0_2_t0b; // @[Map2T.scala 17:7]
  assign O_0_2_t1b = op_O_0_2_t1b; // @[Map2T.scala 17:7]
  assign O_1_0_t0b = op_O_1_0_t0b; // @[Map2T.scala 17:7]
  assign O_1_0_t1b = op_O_1_0_t1b; // @[Map2T.scala 17:7]
  assign O_1_1_t0b = op_O_1_1_t0b; // @[Map2T.scala 17:7]
  assign O_1_1_t1b = op_O_1_1_t1b; // @[Map2T.scala 17:7]
  assign O_1_2_t0b = op_O_1_2_t0b; // @[Map2T.scala 17:7]
  assign O_1_2_t1b = op_O_1_2_t1b; // @[Map2T.scala 17:7]
  assign O_2_0_t0b = op_O_2_0_t0b; // @[Map2T.scala 17:7]
  assign O_2_0_t1b = op_O_2_0_t1b; // @[Map2T.scala 17:7]
  assign O_2_1_t0b = op_O_2_1_t0b; // @[Map2T.scala 17:7]
  assign O_2_1_t1b = op_O_2_1_t1b; // @[Map2T.scala 17:7]
  assign O_2_2_t0b = op_O_2_2_t0b; // @[Map2T.scala 17:7]
  assign O_2_2_t1b = op_O_2_2_t1b; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0_0 = I0_0_0; // @[Map2T.scala 15:11]
  assign op_I0_0_1 = I0_0_1; // @[Map2T.scala 15:11]
  assign op_I0_0_2 = I0_0_2; // @[Map2T.scala 15:11]
  assign op_I0_1_0 = I0_1_0; // @[Map2T.scala 15:11]
  assign op_I0_1_1 = I0_1_1; // @[Map2T.scala 15:11]
  assign op_I0_1_2 = I0_1_2; // @[Map2T.scala 15:11]
  assign op_I0_2_0 = I0_2_0; // @[Map2T.scala 15:11]
  assign op_I0_2_1 = I0_2_1; // @[Map2T.scala 15:11]
  assign op_I0_2_2 = I0_2_2; // @[Map2T.scala 15:11]
endmodule
module Mul(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_t0b,
  input  [7:0] I_t1b,
  output [7:0] O
);
  wire [7:0] BlackBoxMulUInt8_I0; // @[Arithmetic.scala 165:27]
  wire [7:0] BlackBoxMulUInt8_I1; // @[Arithmetic.scala 165:27]
  wire [15:0] BlackBoxMulUInt8_O; // @[Arithmetic.scala 165:27]
  wire  BlackBoxMulUInt8_clk; // @[Arithmetic.scala 165:27]
  reg  _T_1; // @[Arithmetic.scala 217:42]
  reg [31:0] _RAND_0;
  reg  _T_2; // @[Arithmetic.scala 217:34]
  reg [31:0] _RAND_1;
  reg  _T_3; // @[Arithmetic.scala 217:26]
  reg [31:0] _RAND_2;
  BlackBoxMulUInt8 BlackBoxMulUInt8 ( // @[Arithmetic.scala 165:27]
    .I0(BlackBoxMulUInt8_I0),
    .I1(BlackBoxMulUInt8_I1),
    .O(BlackBoxMulUInt8_O),
    .clk(BlackBoxMulUInt8_clk)
  );
  assign valid_down = _T_3; // @[Arithmetic.scala 217:16]
  assign O = BlackBoxMulUInt8_O[7:0]; // @[Arithmetic.scala 168:7]
  assign BlackBoxMulUInt8_I0 = I_t0b; // @[Arithmetic.scala 166:21]
  assign BlackBoxMulUInt8_I1 = I_t1b; // @[Arithmetic.scala 167:21]
  assign BlackBoxMulUInt8_clk = clk; // @[Arithmetic.scala 169:24]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T_1 = _RAND_0[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_2 = _RAND_1[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  _T_3 = _RAND_2[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if (reset) begin
      _T_1 <= 1'h0;
    end else begin
      _T_1 <= valid_up;
    end
    if (reset) begin
      _T_2 <= 1'h0;
    end else begin
      _T_2 <= _T_1;
    end
    if (reset) begin
      _T_3 <= 1'h0;
    end else begin
      _T_3 <= _T_2;
    end
  end
endmodule
module MapS_3(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_t0b,
  input  [7:0] I_0_t1b,
  input  [7:0] I_1_t0b,
  input  [7:0] I_1_t1b,
  input  [7:0] I_2_t0b,
  input  [7:0] I_2_t1b,
  output [7:0] O_0,
  output [7:0] O_1,
  output [7:0] O_2
);
  wire  fst_op_clk; // @[MapS.scala 9:22]
  wire  fst_op_reset; // @[MapS.scala 9:22]
  wire  fst_op_valid_up; // @[MapS.scala 9:22]
  wire  fst_op_valid_down; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_t0b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_t1b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O; // @[MapS.scala 9:22]
  wire  other_ops_0_clk; // @[MapS.scala 10:86]
  wire  other_ops_0_reset; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_O; // @[MapS.scala 10:86]
  wire  other_ops_1_clk; // @[MapS.scala 10:86]
  wire  other_ops_1_reset; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_O; // @[MapS.scala 10:86]
  wire  _T = fst_op_valid_down & other_ops_0_valid_down; // @[MapS.scala 23:83]
  Mul fst_op ( // @[MapS.scala 9:22]
    .clk(fst_op_clk),
    .reset(fst_op_reset),
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I_t0b(fst_op_I_t0b),
    .I_t1b(fst_op_I_t1b),
    .O(fst_op_O)
  );
  Mul other_ops_0 ( // @[MapS.scala 10:86]
    .clk(other_ops_0_clk),
    .reset(other_ops_0_reset),
    .valid_up(other_ops_0_valid_up),
    .valid_down(other_ops_0_valid_down),
    .I_t0b(other_ops_0_I_t0b),
    .I_t1b(other_ops_0_I_t1b),
    .O(other_ops_0_O)
  );
  Mul other_ops_1 ( // @[MapS.scala 10:86]
    .clk(other_ops_1_clk),
    .reset(other_ops_1_reset),
    .valid_up(other_ops_1_valid_up),
    .valid_down(other_ops_1_valid_down),
    .I_t0b(other_ops_1_I_t0b),
    .I_t1b(other_ops_1_I_t1b),
    .O(other_ops_1_O)
  );
  assign valid_down = _T & other_ops_1_valid_down; // @[MapS.scala 23:14]
  assign O_0 = fst_op_O; // @[MapS.scala 17:8]
  assign O_1 = other_ops_0_O; // @[MapS.scala 21:12]
  assign O_2 = other_ops_1_O; // @[MapS.scala 21:12]
  assign fst_op_clk = clk;
  assign fst_op_reset = reset;
  assign fst_op_valid_up = valid_up; // @[MapS.scala 15:19]
  assign fst_op_I_t0b = I_0_t0b; // @[MapS.scala 16:12]
  assign fst_op_I_t1b = I_0_t1b; // @[MapS.scala 16:12]
  assign other_ops_0_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_0_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_0_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_0_I_t0b = I_1_t0b; // @[MapS.scala 20:41]
  assign other_ops_0_I_t1b = I_1_t1b; // @[MapS.scala 20:41]
  assign other_ops_1_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_1_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_1_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_1_I_t0b = I_2_t0b; // @[MapS.scala 20:41]
  assign other_ops_1_I_t1b = I_2_t1b; // @[MapS.scala 20:41]
endmodule
module MapS_4(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_t0b,
  input  [7:0] I_0_0_t1b,
  input  [7:0] I_0_1_t0b,
  input  [7:0] I_0_1_t1b,
  input  [7:0] I_0_2_t0b,
  input  [7:0] I_0_2_t1b,
  input  [7:0] I_1_0_t0b,
  input  [7:0] I_1_0_t1b,
  input  [7:0] I_1_1_t0b,
  input  [7:0] I_1_1_t1b,
  input  [7:0] I_1_2_t0b,
  input  [7:0] I_1_2_t1b,
  input  [7:0] I_2_0_t0b,
  input  [7:0] I_2_0_t1b,
  input  [7:0] I_2_1_t0b,
  input  [7:0] I_2_1_t1b,
  input  [7:0] I_2_2_t0b,
  input  [7:0] I_2_2_t1b,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2,
  output [7:0] O_2_0,
  output [7:0] O_2_1,
  output [7:0] O_2_2
);
  wire  fst_op_clk; // @[MapS.scala 9:22]
  wire  fst_op_reset; // @[MapS.scala 9:22]
  wire  fst_op_valid_up; // @[MapS.scala 9:22]
  wire  fst_op_valid_down; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0_t0b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0_t1b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_1_t0b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_1_t1b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_2_t0b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_2_t1b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_0; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_1; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_2; // @[MapS.scala 9:22]
  wire  other_ops_0_clk; // @[MapS.scala 10:86]
  wire  other_ops_0_reset; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_0_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_0_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_1_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_1_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_2_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_2_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_O_0; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_O_1; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_O_2; // @[MapS.scala 10:86]
  wire  other_ops_1_clk; // @[MapS.scala 10:86]
  wire  other_ops_1_reset; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_0_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_0_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_1_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_1_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_2_t0b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_2_t1b; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_O_0; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_O_1; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_O_2; // @[MapS.scala 10:86]
  wire  _T = fst_op_valid_down & other_ops_0_valid_down; // @[MapS.scala 23:83]
  MapS_3 fst_op ( // @[MapS.scala 9:22]
    .clk(fst_op_clk),
    .reset(fst_op_reset),
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I_0_t0b(fst_op_I_0_t0b),
    .I_0_t1b(fst_op_I_0_t1b),
    .I_1_t0b(fst_op_I_1_t0b),
    .I_1_t1b(fst_op_I_1_t1b),
    .I_2_t0b(fst_op_I_2_t0b),
    .I_2_t1b(fst_op_I_2_t1b),
    .O_0(fst_op_O_0),
    .O_1(fst_op_O_1),
    .O_2(fst_op_O_2)
  );
  MapS_3 other_ops_0 ( // @[MapS.scala 10:86]
    .clk(other_ops_0_clk),
    .reset(other_ops_0_reset),
    .valid_up(other_ops_0_valid_up),
    .valid_down(other_ops_0_valid_down),
    .I_0_t0b(other_ops_0_I_0_t0b),
    .I_0_t1b(other_ops_0_I_0_t1b),
    .I_1_t0b(other_ops_0_I_1_t0b),
    .I_1_t1b(other_ops_0_I_1_t1b),
    .I_2_t0b(other_ops_0_I_2_t0b),
    .I_2_t1b(other_ops_0_I_2_t1b),
    .O_0(other_ops_0_O_0),
    .O_1(other_ops_0_O_1),
    .O_2(other_ops_0_O_2)
  );
  MapS_3 other_ops_1 ( // @[MapS.scala 10:86]
    .clk(other_ops_1_clk),
    .reset(other_ops_1_reset),
    .valid_up(other_ops_1_valid_up),
    .valid_down(other_ops_1_valid_down),
    .I_0_t0b(other_ops_1_I_0_t0b),
    .I_0_t1b(other_ops_1_I_0_t1b),
    .I_1_t0b(other_ops_1_I_1_t0b),
    .I_1_t1b(other_ops_1_I_1_t1b),
    .I_2_t0b(other_ops_1_I_2_t0b),
    .I_2_t1b(other_ops_1_I_2_t1b),
    .O_0(other_ops_1_O_0),
    .O_1(other_ops_1_O_1),
    .O_2(other_ops_1_O_2)
  );
  assign valid_down = _T & other_ops_1_valid_down; // @[MapS.scala 23:14]
  assign O_0_0 = fst_op_O_0; // @[MapS.scala 17:8]
  assign O_0_1 = fst_op_O_1; // @[MapS.scala 17:8]
  assign O_0_2 = fst_op_O_2; // @[MapS.scala 17:8]
  assign O_1_0 = other_ops_0_O_0; // @[MapS.scala 21:12]
  assign O_1_1 = other_ops_0_O_1; // @[MapS.scala 21:12]
  assign O_1_2 = other_ops_0_O_2; // @[MapS.scala 21:12]
  assign O_2_0 = other_ops_1_O_0; // @[MapS.scala 21:12]
  assign O_2_1 = other_ops_1_O_1; // @[MapS.scala 21:12]
  assign O_2_2 = other_ops_1_O_2; // @[MapS.scala 21:12]
  assign fst_op_clk = clk;
  assign fst_op_reset = reset;
  assign fst_op_valid_up = valid_up; // @[MapS.scala 15:19]
  assign fst_op_I_0_t0b = I_0_0_t0b; // @[MapS.scala 16:12]
  assign fst_op_I_0_t1b = I_0_0_t1b; // @[MapS.scala 16:12]
  assign fst_op_I_1_t0b = I_0_1_t0b; // @[MapS.scala 16:12]
  assign fst_op_I_1_t1b = I_0_1_t1b; // @[MapS.scala 16:12]
  assign fst_op_I_2_t0b = I_0_2_t0b; // @[MapS.scala 16:12]
  assign fst_op_I_2_t1b = I_0_2_t1b; // @[MapS.scala 16:12]
  assign other_ops_0_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_0_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_0_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_0_I_0_t0b = I_1_0_t0b; // @[MapS.scala 20:41]
  assign other_ops_0_I_0_t1b = I_1_0_t1b; // @[MapS.scala 20:41]
  assign other_ops_0_I_1_t0b = I_1_1_t0b; // @[MapS.scala 20:41]
  assign other_ops_0_I_1_t1b = I_1_1_t1b; // @[MapS.scala 20:41]
  assign other_ops_0_I_2_t0b = I_1_2_t0b; // @[MapS.scala 20:41]
  assign other_ops_0_I_2_t1b = I_1_2_t1b; // @[MapS.scala 20:41]
  assign other_ops_1_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_1_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_1_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_1_I_0_t0b = I_2_0_t0b; // @[MapS.scala 20:41]
  assign other_ops_1_I_0_t1b = I_2_0_t1b; // @[MapS.scala 20:41]
  assign other_ops_1_I_1_t0b = I_2_1_t0b; // @[MapS.scala 20:41]
  assign other_ops_1_I_1_t1b = I_2_1_t1b; // @[MapS.scala 20:41]
  assign other_ops_1_I_2_t0b = I_2_2_t0b; // @[MapS.scala 20:41]
  assign other_ops_1_I_2_t1b = I_2_2_t1b; // @[MapS.scala 20:41]
endmodule
module MapT_7(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0_t0b,
  input  [7:0] I_0_0_t1b,
  input  [7:0] I_0_1_t0b,
  input  [7:0] I_0_1_t1b,
  input  [7:0] I_0_2_t0b,
  input  [7:0] I_0_2_t1b,
  input  [7:0] I_1_0_t0b,
  input  [7:0] I_1_0_t1b,
  input  [7:0] I_1_1_t0b,
  input  [7:0] I_1_1_t1b,
  input  [7:0] I_1_2_t0b,
  input  [7:0] I_1_2_t1b,
  input  [7:0] I_2_0_t0b,
  input  [7:0] I_2_0_t1b,
  input  [7:0] I_2_1_t0b,
  input  [7:0] I_2_1_t1b,
  input  [7:0] I_2_2_t0b,
  input  [7:0] I_2_2_t1b,
  output [7:0] O_0_0,
  output [7:0] O_0_1,
  output [7:0] O_0_2,
  output [7:0] O_1_0,
  output [7:0] O_1_1,
  output [7:0] O_1_2,
  output [7:0] O_2_0,
  output [7:0] O_2_1,
  output [7:0] O_2_2
);
  wire  op_clk; // @[MapT.scala 8:20]
  wire  op_reset; // @[MapT.scala 8:20]
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_0_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_0_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_1_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_1_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_2_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_2_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_0_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_0_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_1_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_1_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_2_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_2_t1b; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_1; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_2; // @[MapT.scala 8:20]
  MapS_4 op ( // @[MapT.scala 8:20]
    .clk(op_clk),
    .reset(op_reset),
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0_t0b(op_I_0_0_t0b),
    .I_0_0_t1b(op_I_0_0_t1b),
    .I_0_1_t0b(op_I_0_1_t0b),
    .I_0_1_t1b(op_I_0_1_t1b),
    .I_0_2_t0b(op_I_0_2_t0b),
    .I_0_2_t1b(op_I_0_2_t1b),
    .I_1_0_t0b(op_I_1_0_t0b),
    .I_1_0_t1b(op_I_1_0_t1b),
    .I_1_1_t0b(op_I_1_1_t0b),
    .I_1_1_t1b(op_I_1_1_t1b),
    .I_1_2_t0b(op_I_1_2_t0b),
    .I_1_2_t1b(op_I_1_2_t1b),
    .I_2_0_t0b(op_I_2_0_t0b),
    .I_2_0_t1b(op_I_2_0_t1b),
    .I_2_1_t0b(op_I_2_1_t0b),
    .I_2_1_t1b(op_I_2_1_t1b),
    .I_2_2_t0b(op_I_2_2_t0b),
    .I_2_2_t1b(op_I_2_2_t1b),
    .O_0_0(op_O_0_0),
    .O_0_1(op_O_0_1),
    .O_0_2(op_O_0_2),
    .O_1_0(op_O_1_0),
    .O_1_1(op_O_1_1),
    .O_1_2(op_O_1_2),
    .O_2_0(op_O_2_0),
    .O_2_1(op_O_2_1),
    .O_2_2(op_O_2_2)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0 = op_O_0_0; // @[MapT.scala 15:7]
  assign O_0_1 = op_O_0_1; // @[MapT.scala 15:7]
  assign O_0_2 = op_O_0_2; // @[MapT.scala 15:7]
  assign O_1_0 = op_O_1_0; // @[MapT.scala 15:7]
  assign O_1_1 = op_O_1_1; // @[MapT.scala 15:7]
  assign O_1_2 = op_O_1_2; // @[MapT.scala 15:7]
  assign O_2_0 = op_O_2_0; // @[MapT.scala 15:7]
  assign O_2_1 = op_O_2_1; // @[MapT.scala 15:7]
  assign O_2_2 = op_O_2_2; // @[MapT.scala 15:7]
  assign op_clk = clk;
  assign op_reset = reset;
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0_t0b = I_0_0_t0b; // @[MapT.scala 14:10]
  assign op_I_0_0_t1b = I_0_0_t1b; // @[MapT.scala 14:10]
  assign op_I_0_1_t0b = I_0_1_t0b; // @[MapT.scala 14:10]
  assign op_I_0_1_t1b = I_0_1_t1b; // @[MapT.scala 14:10]
  assign op_I_0_2_t0b = I_0_2_t0b; // @[MapT.scala 14:10]
  assign op_I_0_2_t1b = I_0_2_t1b; // @[MapT.scala 14:10]
  assign op_I_1_0_t0b = I_1_0_t0b; // @[MapT.scala 14:10]
  assign op_I_1_0_t1b = I_1_0_t1b; // @[MapT.scala 14:10]
  assign op_I_1_1_t0b = I_1_1_t0b; // @[MapT.scala 14:10]
  assign op_I_1_1_t1b = I_1_1_t1b; // @[MapT.scala 14:10]
  assign op_I_1_2_t0b = I_1_2_t0b; // @[MapT.scala 14:10]
  assign op_I_1_2_t1b = I_1_2_t1b; // @[MapT.scala 14:10]
  assign op_I_2_0_t0b = I_2_0_t0b; // @[MapT.scala 14:10]
  assign op_I_2_0_t1b = I_2_0_t1b; // @[MapT.scala 14:10]
  assign op_I_2_1_t0b = I_2_1_t0b; // @[MapT.scala 14:10]
  assign op_I_2_1_t1b = I_2_1_t1b; // @[MapT.scala 14:10]
  assign op_I_2_2_t0b = I_2_2_t0b; // @[MapT.scala 14:10]
  assign op_I_2_2_t1b = I_2_2_t1b; // @[MapT.scala 14:10]
endmodule
module AddNoValid(
  input  [7:0] I_t0b,
  input  [7:0] I_t1b,
  output [7:0] O
);
  assign O = I_t0b + I_t1b; // @[Arithmetic.scala 122:7]
endmodule
module ReduceS(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  input  [7:0] I_1,
  input  [7:0] I_2,
  output [7:0] O_0
);
  wire [7:0] AddNoValid_I_t0b; // @[ReduceS.scala 20:43]
  wire [7:0] AddNoValid_I_t1b; // @[ReduceS.scala 20:43]
  wire [7:0] AddNoValid_O; // @[ReduceS.scala 20:43]
  wire [7:0] AddNoValid_1_I_t0b; // @[ReduceS.scala 20:43]
  wire [7:0] AddNoValid_1_I_t1b; // @[ReduceS.scala 20:43]
  wire [7:0] AddNoValid_1_O; // @[ReduceS.scala 20:43]
  reg [7:0] _T; // @[ReduceS.scala 27:24]
  reg [31:0] _RAND_0;
  reg [7:0] _T_1; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_1;
  reg [7:0] _T_2; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_2;
  reg [7:0] _T_3; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_3;
  reg  _T_4; // @[ReduceS.scala 47:32]
  reg [31:0] _RAND_4;
  reg  _T_5; // @[ReduceS.scala 47:24]
  reg [31:0] _RAND_5;
  AddNoValid AddNoValid ( // @[ReduceS.scala 20:43]
    .I_t0b(AddNoValid_I_t0b),
    .I_t1b(AddNoValid_I_t1b),
    .O(AddNoValid_O)
  );
  AddNoValid AddNoValid_1 ( // @[ReduceS.scala 20:43]
    .I_t0b(AddNoValid_1_I_t0b),
    .I_t1b(AddNoValid_1_I_t1b),
    .O(AddNoValid_1_O)
  );
  assign valid_down = _T_5; // @[ReduceS.scala 47:14]
  assign O_0 = _T; // @[ReduceS.scala 27:14]
  assign AddNoValid_I_t0b = _T_3; // @[ReduceS.scala 43:18]
  assign AddNoValid_I_t1b = AddNoValid_1_O; // @[ReduceS.scala 36:18]
  assign AddNoValid_1_I_t0b = _T_2; // @[ReduceS.scala 43:18]
  assign AddNoValid_1_I_t1b = _T_1; // @[ReduceS.scala 43:18]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T = _RAND_0[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_1 = _RAND_1[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  _T_2 = _RAND_2[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_3 = {1{`RANDOM}};
  _T_3 = _RAND_3[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_4 = {1{`RANDOM}};
  _T_4 = _RAND_4[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_5 = {1{`RANDOM}};
  _T_5 = _RAND_5[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    _T <= AddNoValid_O;
    _T_1 <= I_0;
    _T_2 <= I_1;
    _T_3 <= I_2;
    if (reset) begin
      _T_4 <= 1'h0;
    end else begin
      _T_4 <= valid_up;
    end
    _T_5 <= _T_4;
  end
endmodule
module MapS_5(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  input  [7:0] I_1_0,
  input  [7:0] I_1_1,
  input  [7:0] I_1_2,
  input  [7:0] I_2_0,
  input  [7:0] I_2_1,
  input  [7:0] I_2_2,
  output [7:0] O_0_0,
  output [7:0] O_1_0,
  output [7:0] O_2_0
);
  wire  fst_op_clk; // @[MapS.scala 9:22]
  wire  fst_op_reset; // @[MapS.scala 9:22]
  wire  fst_op_valid_up; // @[MapS.scala 9:22]
  wire  fst_op_valid_down; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_0; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_1; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_2; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O_0; // @[MapS.scala 9:22]
  wire  other_ops_0_clk; // @[MapS.scala 10:86]
  wire  other_ops_0_reset; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_0_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_0; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_1; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_I_2; // @[MapS.scala 10:86]
  wire [7:0] other_ops_0_O_0; // @[MapS.scala 10:86]
  wire  other_ops_1_clk; // @[MapS.scala 10:86]
  wire  other_ops_1_reset; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_up; // @[MapS.scala 10:86]
  wire  other_ops_1_valid_down; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_0; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_1; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_I_2; // @[MapS.scala 10:86]
  wire [7:0] other_ops_1_O_0; // @[MapS.scala 10:86]
  wire  _T = fst_op_valid_down & other_ops_0_valid_down; // @[MapS.scala 23:83]
  ReduceS fst_op ( // @[MapS.scala 9:22]
    .clk(fst_op_clk),
    .reset(fst_op_reset),
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I_0(fst_op_I_0),
    .I_1(fst_op_I_1),
    .I_2(fst_op_I_2),
    .O_0(fst_op_O_0)
  );
  ReduceS other_ops_0 ( // @[MapS.scala 10:86]
    .clk(other_ops_0_clk),
    .reset(other_ops_0_reset),
    .valid_up(other_ops_0_valid_up),
    .valid_down(other_ops_0_valid_down),
    .I_0(other_ops_0_I_0),
    .I_1(other_ops_0_I_1),
    .I_2(other_ops_0_I_2),
    .O_0(other_ops_0_O_0)
  );
  ReduceS other_ops_1 ( // @[MapS.scala 10:86]
    .clk(other_ops_1_clk),
    .reset(other_ops_1_reset),
    .valid_up(other_ops_1_valid_up),
    .valid_down(other_ops_1_valid_down),
    .I_0(other_ops_1_I_0),
    .I_1(other_ops_1_I_1),
    .I_2(other_ops_1_I_2),
    .O_0(other_ops_1_O_0)
  );
  assign valid_down = _T & other_ops_1_valid_down; // @[MapS.scala 23:14]
  assign O_0_0 = fst_op_O_0; // @[MapS.scala 17:8]
  assign O_1_0 = other_ops_0_O_0; // @[MapS.scala 21:12]
  assign O_2_0 = other_ops_1_O_0; // @[MapS.scala 21:12]
  assign fst_op_clk = clk;
  assign fst_op_reset = reset;
  assign fst_op_valid_up = valid_up; // @[MapS.scala 15:19]
  assign fst_op_I_0 = I_0_0; // @[MapS.scala 16:12]
  assign fst_op_I_1 = I_0_1; // @[MapS.scala 16:12]
  assign fst_op_I_2 = I_0_2; // @[MapS.scala 16:12]
  assign other_ops_0_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_0_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_0_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_0_I_0 = I_1_0; // @[MapS.scala 20:41]
  assign other_ops_0_I_1 = I_1_1; // @[MapS.scala 20:41]
  assign other_ops_0_I_2 = I_1_2; // @[MapS.scala 20:41]
  assign other_ops_1_clk = clk; // @[MapS.scala 10:86]
  assign other_ops_1_reset = reset; // @[MapS.scala 10:86]
  assign other_ops_1_valid_up = valid_up; // @[MapS.scala 19:39]
  assign other_ops_1_I_0 = I_2_0; // @[MapS.scala 20:41]
  assign other_ops_1_I_1 = I_2_1; // @[MapS.scala 20:41]
  assign other_ops_1_I_2 = I_2_2; // @[MapS.scala 20:41]
endmodule
module MapT_8(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  input  [7:0] I_1_0,
  input  [7:0] I_1_1,
  input  [7:0] I_1_2,
  input  [7:0] I_2_0,
  input  [7:0] I_2_1,
  input  [7:0] I_2_2,
  output [7:0] O_0_0,
  output [7:0] O_1_0,
  output [7:0] O_2_0
);
  wire  op_clk; // @[MapT.scala 8:20]
  wire  op_reset; // @[MapT.scala 8:20]
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_2_0; // @[MapT.scala 8:20]
  MapS_5 op ( // @[MapT.scala 8:20]
    .clk(op_clk),
    .reset(op_reset),
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0(op_I_0_0),
    .I_0_1(op_I_0_1),
    .I_0_2(op_I_0_2),
    .I_1_0(op_I_1_0),
    .I_1_1(op_I_1_1),
    .I_1_2(op_I_1_2),
    .I_2_0(op_I_2_0),
    .I_2_1(op_I_2_1),
    .I_2_2(op_I_2_2),
    .O_0_0(op_O_0_0),
    .O_1_0(op_O_1_0),
    .O_2_0(op_O_2_0)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0 = op_O_0_0; // @[MapT.scala 15:7]
  assign O_1_0 = op_O_1_0; // @[MapT.scala 15:7]
  assign O_2_0 = op_O_2_0; // @[MapT.scala 15:7]
  assign op_clk = clk;
  assign op_reset = reset;
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0 = I_0_0; // @[MapT.scala 14:10]
  assign op_I_0_1 = I_0_1; // @[MapT.scala 14:10]
  assign op_I_0_2 = I_0_2; // @[MapT.scala 14:10]
  assign op_I_1_0 = I_1_0; // @[MapT.scala 14:10]
  assign op_I_1_1 = I_1_1; // @[MapT.scala 14:10]
  assign op_I_1_2 = I_1_2; // @[MapT.scala 14:10]
  assign op_I_2_0 = I_2_0; // @[MapT.scala 14:10]
  assign op_I_2_1 = I_2_1; // @[MapT.scala 14:10]
  assign op_I_2_2 = I_2_2; // @[MapT.scala 14:10]
endmodule
module MapSNoValid(
  input  [7:0] I_0_t0b,
  input  [7:0] I_0_t1b,
  output [7:0] O_0
);
  wire [7:0] fst_op_I_t0b; // @[MapS.scala 28:22]
  wire [7:0] fst_op_I_t1b; // @[MapS.scala 28:22]
  wire [7:0] fst_op_O; // @[MapS.scala 28:22]
  AddNoValid fst_op ( // @[MapS.scala 28:22]
    .I_t0b(fst_op_I_t0b),
    .I_t1b(fst_op_I_t1b),
    .O(fst_op_O)
  );
  assign O_0 = fst_op_O; // @[MapS.scala 35:8]
  assign fst_op_I_t0b = I_0_t0b; // @[MapS.scala 34:12]
  assign fst_op_I_t1b = I_0_t1b; // @[MapS.scala 34:12]
endmodule
module ReduceS_1(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_1_0,
  input  [7:0] I_2_0,
  output [7:0] O_0_0
);
  wire [7:0] MapSNoValid_I_0_t0b; // @[ReduceS.scala 20:43]
  wire [7:0] MapSNoValid_I_0_t1b; // @[ReduceS.scala 20:43]
  wire [7:0] MapSNoValid_O_0; // @[ReduceS.scala 20:43]
  wire [7:0] MapSNoValid_1_I_0_t0b; // @[ReduceS.scala 20:43]
  wire [7:0] MapSNoValid_1_I_0_t1b; // @[ReduceS.scala 20:43]
  wire [7:0] MapSNoValid_1_O_0; // @[ReduceS.scala 20:43]
  reg [7:0] _T_0; // @[ReduceS.scala 27:24]
  reg [31:0] _RAND_0;
  reg [7:0] _T_1_0; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_1;
  reg [7:0] _T_2_0; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_2;
  reg [7:0] _T_3_0; // @[ReduceS.scala 43:46]
  reg [31:0] _RAND_3;
  reg  _T_4; // @[ReduceS.scala 47:32]
  reg [31:0] _RAND_4;
  reg  _T_5; // @[ReduceS.scala 47:24]
  reg [31:0] _RAND_5;
  MapSNoValid MapSNoValid ( // @[ReduceS.scala 20:43]
    .I_0_t0b(MapSNoValid_I_0_t0b),
    .I_0_t1b(MapSNoValid_I_0_t1b),
    .O_0(MapSNoValid_O_0)
  );
  MapSNoValid MapSNoValid_1 ( // @[ReduceS.scala 20:43]
    .I_0_t0b(MapSNoValid_1_I_0_t0b),
    .I_0_t1b(MapSNoValid_1_I_0_t1b),
    .O_0(MapSNoValid_1_O_0)
  );
  assign valid_down = _T_5; // @[ReduceS.scala 47:14]
  assign O_0_0 = _T_0; // @[ReduceS.scala 27:14]
  assign MapSNoValid_I_0_t0b = _T_1_0; // @[ReduceS.scala 43:18]
  assign MapSNoValid_I_0_t1b = MapSNoValid_1_O_0; // @[ReduceS.scala 36:18]
  assign MapSNoValid_1_I_0_t0b = _T_3_0; // @[ReduceS.scala 43:18]
  assign MapSNoValid_1_I_0_t1b = _T_2_0; // @[ReduceS.scala 43:18]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T_0 = _RAND_0[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_1_0 = _RAND_1[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  _T_2_0 = _RAND_2[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_3 = {1{`RANDOM}};
  _T_3_0 = _RAND_3[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_4 = {1{`RANDOM}};
  _T_4 = _RAND_4[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_5 = {1{`RANDOM}};
  _T_5 = _RAND_5[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    _T_0 <= MapSNoValid_O_0;
    _T_1_0 <= I_0_0;
    _T_2_0 <= I_1_0;
    _T_3_0 <= I_2_0;
    if (reset) begin
      _T_4 <= 1'h0;
    end else begin
      _T_4 <= valid_up;
    end
    _T_5 <= _T_4;
  end
endmodule
module MapT_9(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_1_0,
  input  [7:0] I_2_0,
  output [7:0] O_0_0
);
  wire  op_clk; // @[MapT.scala 8:20]
  wire  op_reset; // @[MapT.scala 8:20]
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_O_0_0; // @[MapT.scala 8:20]
  ReduceS_1 op ( // @[MapT.scala 8:20]
    .clk(op_clk),
    .reset(op_reset),
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0(op_I_0_0),
    .I_1_0(op_I_1_0),
    .I_2_0(op_I_2_0),
    .O_0_0(op_O_0_0)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0_0 = op_O_0_0; // @[MapT.scala 15:7]
  assign op_clk = clk;
  assign op_reset = reset;
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0 = I_0_0; // @[MapT.scala 14:10]
  assign op_I_1_0 = I_1_0; // @[MapT.scala 14:10]
  assign op_I_2_0 = I_2_0; // @[MapT.scala 14:10]
endmodule
module ReduceT(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  output [7:0] O_0_0
);
  reg [7:0] undelayed_out_0_0; // @[ReduceT.scala 17:29]
  reg [31:0] _RAND_0;
  reg  _T_1; // @[ReduceT.scala 18:34]
  reg [31:0] _RAND_1;
  reg  _T_2; // @[ReduceT.scala 18:26]
  reg [31:0] _RAND_2;
  reg [7:0] _T_3_0_0; // @[ReduceT.scala 56:15]
  reg [31:0] _RAND_3;
  assign valid_down = _T_2; // @[ReduceT.scala 18:16]
  assign O_0_0 = _T_3_0_0; // @[ReduceT.scala 56:5]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  undelayed_out_0_0 = _RAND_0[7:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_1 = _RAND_1[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  _T_2 = _RAND_2[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_3 = {1{`RANDOM}};
  _T_3_0_0 = _RAND_3[7:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    undelayed_out_0_0 <= I_0_0;
    if (reset) begin
      _T_1 <= 1'h0;
    end else begin
      _T_1 <= valid_up;
    end
    if (reset) begin
      _T_2 <= 1'h0;
    end else begin
      _T_2 <= _T_1;
    end
    _T_3_0_0 <= undelayed_out_0_0;
  end
endmodule
module Passthrough_2(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  output [7:0] O_0
);
  assign valid_down = valid_up; // @[Passthrough.scala 18:14]
  assign O_0 = I_0_0; // @[Passthrough.scala 17:68]
endmodule
module InitialDelayCounter_1(
  input   clk,
  input   reset,
  output  valid_down
);
  reg [3:0] value; // @[InitialDelayCounter.scala 8:34]
  reg [31:0] _RAND_0;
  wire  _T_1 = value < 4'ha; // @[InitialDelayCounter.scala 17:17]
  wire [3:0] _T_4 = value + 4'h1; // @[InitialDelayCounter.scala 17:53]
  assign valid_down = value == 4'ha; // @[InitialDelayCounter.scala 16:16]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  value = _RAND_0[3:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if (reset) begin
      value <= 4'h0;
    end else if (_T_1) begin
      value <= _T_4;
    end
  end
endmodule
module AtomTuple_1(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0,
  output [7:0] O_t0b
);
  assign valid_down = valid_up; // @[Tuple.scala 51:14]
  assign O_t0b = I0; // @[Tuple.scala 49:9]
endmodule
module Map2S_10(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  output [7:0] O_0_t0b
);
  wire  fst_op_valid_up; // @[Map2S.scala 9:22]
  wire  fst_op_valid_down; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_I0; // @[Map2S.scala 9:22]
  wire [7:0] fst_op_O_t0b; // @[Map2S.scala 9:22]
  AtomTuple_1 fst_op ( // @[Map2S.scala 9:22]
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I0(fst_op_I0),
    .O_t0b(fst_op_O_t0b)
  );
  assign valid_down = fst_op_valid_down; // @[Map2S.scala 26:14]
  assign O_0_t0b = fst_op_O_t0b; // @[Map2S.scala 19:8]
  assign fst_op_valid_up = valid_up; // @[Map2S.scala 16:19]
  assign fst_op_I0 = I0_0; // @[Map2S.scala 17:13]
endmodule
module Map2T_9(
  input        valid_up,
  output       valid_down,
  input  [7:0] I0_0,
  output [7:0] O_0_t0b
);
  wire  op_valid_up; // @[Map2T.scala 8:20]
  wire  op_valid_down; // @[Map2T.scala 8:20]
  wire [7:0] op_I0_0; // @[Map2T.scala 8:20]
  wire [7:0] op_O_0_t0b; // @[Map2T.scala 8:20]
  Map2S_10 op ( // @[Map2T.scala 8:20]
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I0_0(op_I0_0),
    .O_0_t0b(op_O_0_t0b)
  );
  assign valid_down = op_valid_down; // @[Map2T.scala 18:16]
  assign O_0_t0b = op_O_0_t0b; // @[Map2T.scala 17:7]
  assign op_valid_up = valid_up; // @[Map2T.scala 14:17]
  assign op_I0_0 = I0_0; // @[Map2T.scala 15:11]
endmodule
module Div(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_t0b,
  output [7:0] O
);
  wire [7:0] BlackBoxMulUInt8_I0; // @[Arithmetic.scala 342:27]
  wire [7:0] BlackBoxMulUInt8_I1; // @[Arithmetic.scala 342:27]
  wire [15:0] BlackBoxMulUInt8_O; // @[Arithmetic.scala 342:27]
  wire  BlackBoxMulUInt8_clk; // @[Arithmetic.scala 342:27]
  reg  _T_3; // @[Arithmetic.scala 370:42]
  reg [31:0] _RAND_0;
  reg  _T_4; // @[Arithmetic.scala 370:34]
  reg [31:0] _RAND_1;
  reg  _T_5; // @[Arithmetic.scala 370:26]
  reg [31:0] _RAND_2;
  BlackBoxMulUInt8 BlackBoxMulUInt8 ( // @[Arithmetic.scala 342:27]
    .I0(BlackBoxMulUInt8_I0),
    .I1(BlackBoxMulUInt8_I1),
    .O(BlackBoxMulUInt8_O),
    .clk(BlackBoxMulUInt8_clk)
  );
  assign valid_down = _T_5; // @[Arithmetic.scala 370:16]
  assign O = BlackBoxMulUInt8_O[14:7]; // @[Arithmetic.scala 345:7]
  assign BlackBoxMulUInt8_I0 = I_t0b; // @[Arithmetic.scala 343:21]
  assign BlackBoxMulUInt8_I1 = 8'h8; // @[Arithmetic.scala 344:21]
  assign BlackBoxMulUInt8_clk = clk; // @[Arithmetic.scala 346:24]
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
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
  `ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  _T_3 = _RAND_0[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_1 = {1{`RANDOM}};
  _T_4 = _RAND_1[0:0];
  `endif // RANDOMIZE_REG_INIT
  `ifdef RANDOMIZE_REG_INIT
  _RAND_2 = {1{`RANDOM}};
  _T_5 = _RAND_2[0:0];
  `endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`endif // SYNTHESIS
  always @(posedge clk) begin
    if (reset) begin
      _T_3 <= 1'h0;
    end else begin
      _T_3 <= valid_up;
    end
    if (reset) begin
      _T_4 <= 1'h0;
    end else begin
      _T_4 <= _T_3;
    end
    if (reset) begin
      _T_5 <= 1'h0;
    end else begin
      _T_5 <= _T_4;
    end
  end
endmodule
module MapS_6(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_t0b,
  output [7:0] O_0
);
  wire  fst_op_clk; // @[MapS.scala 9:22]
  wire  fst_op_reset; // @[MapS.scala 9:22]
  wire  fst_op_valid_up; // @[MapS.scala 9:22]
  wire  fst_op_valid_down; // @[MapS.scala 9:22]
  wire [7:0] fst_op_I_t0b; // @[MapS.scala 9:22]
  wire [7:0] fst_op_O; // @[MapS.scala 9:22]
  Div fst_op ( // @[MapS.scala 9:22]
    .clk(fst_op_clk),
    .reset(fst_op_reset),
    .valid_up(fst_op_valid_up),
    .valid_down(fst_op_valid_down),
    .I_t0b(fst_op_I_t0b),
    .O(fst_op_O)
  );
  assign valid_down = fst_op_valid_down; // @[MapS.scala 23:14]
  assign O_0 = fst_op_O; // @[MapS.scala 17:8]
  assign fst_op_clk = clk;
  assign fst_op_reset = reset;
  assign fst_op_valid_up = valid_up; // @[MapS.scala 15:19]
  assign fst_op_I_t0b = I_0_t0b; // @[MapS.scala 16:12]
endmodule
module MapT_10(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_t0b,
  output [7:0] O_0
);
  wire  op_clk; // @[MapT.scala 8:20]
  wire  op_reset; // @[MapT.scala 8:20]
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_t0b; // @[MapT.scala 8:20]
  wire [7:0] op_O_0; // @[MapT.scala 8:20]
  MapS_6 op ( // @[MapT.scala 8:20]
    .clk(op_clk),
    .reset(op_reset),
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_t0b(op_I_0_t0b),
    .O_0(op_O_0)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0 = op_O_0; // @[MapT.scala 15:7]
  assign op_clk = clk;
  assign op_reset = reset;
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_t0b = I_0_t0b; // @[MapT.scala 14:10]
endmodule
module Module_0(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  input  [7:0] I_1_0,
  input  [7:0] I_1_1,
  input  [7:0] I_1_2,
  input  [7:0] I_2_0,
  input  [7:0] I_2_1,
  input  [7:0] I_2_2,
  output [7:0] O_0
);
  wire  InitialDelayCounter_clk; // @[Const.scala 11:33]
  wire  InitialDelayCounter_reset; // @[Const.scala 11:33]
  wire  InitialDelayCounter_valid_down; // @[Const.scala 11:33]
  wire  n103_valid_up; // @[Top.scala 18:22]
  wire  n103_valid_down; // @[Top.scala 18:22]
  wire [7:0] n103_I0_0_0; // @[Top.scala 18:22]
  wire [7:0] n103_I0_0_1; // @[Top.scala 18:22]
  wire [7:0] n103_I0_0_2; // @[Top.scala 18:22]
  wire [7:0] n103_I0_1_0; // @[Top.scala 18:22]
  wire [7:0] n103_I0_1_1; // @[Top.scala 18:22]
  wire [7:0] n103_I0_1_2; // @[Top.scala 18:22]
  wire [7:0] n103_I0_2_0; // @[Top.scala 18:22]
  wire [7:0] n103_I0_2_1; // @[Top.scala 18:22]
  wire [7:0] n103_I0_2_2; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_0_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_0_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_1_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_1_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_2_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_0_2_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_0_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_0_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_1_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_1_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_2_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_1_2_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_0_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_0_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_1_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_1_t1b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_2_t0b; // @[Top.scala 18:22]
  wire [7:0] n103_O_2_2_t1b; // @[Top.scala 18:22]
  wire  n119_clk; // @[Top.scala 22:22]
  wire  n119_reset; // @[Top.scala 22:22]
  wire  n119_valid_up; // @[Top.scala 22:22]
  wire  n119_valid_down; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_0_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_0_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_1_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_1_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_2_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_0_2_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_0_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_0_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_1_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_1_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_2_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_1_2_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_0_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_0_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_1_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_1_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_2_t0b; // @[Top.scala 22:22]
  wire [7:0] n119_I_2_2_t1b; // @[Top.scala 22:22]
  wire [7:0] n119_O_0_0; // @[Top.scala 22:22]
  wire [7:0] n119_O_0_1; // @[Top.scala 22:22]
  wire [7:0] n119_O_0_2; // @[Top.scala 22:22]
  wire [7:0] n119_O_1_0; // @[Top.scala 22:22]
  wire [7:0] n119_O_1_1; // @[Top.scala 22:22]
  wire [7:0] n119_O_1_2; // @[Top.scala 22:22]
  wire [7:0] n119_O_2_0; // @[Top.scala 22:22]
  wire [7:0] n119_O_2_1; // @[Top.scala 22:22]
  wire [7:0] n119_O_2_2; // @[Top.scala 22:22]
  wire  n126_clk; // @[Top.scala 25:22]
  wire  n126_reset; // @[Top.scala 25:22]
  wire  n126_valid_up; // @[Top.scala 25:22]
  wire  n126_valid_down; // @[Top.scala 25:22]
  wire [7:0] n126_I_0_0; // @[Top.scala 25:22]
  wire [7:0] n126_I_0_1; // @[Top.scala 25:22]
  wire [7:0] n126_I_0_2; // @[Top.scala 25:22]
  wire [7:0] n126_I_1_0; // @[Top.scala 25:22]
  wire [7:0] n126_I_1_1; // @[Top.scala 25:22]
  wire [7:0] n126_I_1_2; // @[Top.scala 25:22]
  wire [7:0] n126_I_2_0; // @[Top.scala 25:22]
  wire [7:0] n126_I_2_1; // @[Top.scala 25:22]
  wire [7:0] n126_I_2_2; // @[Top.scala 25:22]
  wire [7:0] n126_O_0_0; // @[Top.scala 25:22]
  wire [7:0] n126_O_1_0; // @[Top.scala 25:22]
  wire [7:0] n126_O_2_0; // @[Top.scala 25:22]
  wire  n133_clk; // @[Top.scala 28:22]
  wire  n133_reset; // @[Top.scala 28:22]
  wire  n133_valid_up; // @[Top.scala 28:22]
  wire  n133_valid_down; // @[Top.scala 28:22]
  wire [7:0] n133_I_0_0; // @[Top.scala 28:22]
  wire [7:0] n133_I_1_0; // @[Top.scala 28:22]
  wire [7:0] n133_I_2_0; // @[Top.scala 28:22]
  wire [7:0] n133_O_0_0; // @[Top.scala 28:22]
  wire  n136_clk; // @[Top.scala 31:22]
  wire  n136_reset; // @[Top.scala 31:22]
  wire  n136_valid_up; // @[Top.scala 31:22]
  wire  n136_valid_down; // @[Top.scala 31:22]
  wire [7:0] n136_I_0_0; // @[Top.scala 31:22]
  wire [7:0] n136_O_0_0; // @[Top.scala 31:22]
  wire  n137_valid_up; // @[Top.scala 34:22]
  wire  n137_valid_down; // @[Top.scala 34:22]
  wire [7:0] n137_I_0_0; // @[Top.scala 34:22]
  wire [7:0] n137_O_0; // @[Top.scala 34:22]
  wire  InitialDelayCounter_1_clk; // @[Const.scala 11:33]
  wire  InitialDelayCounter_1_reset; // @[Const.scala 11:33]
  wire  InitialDelayCounter_1_valid_down; // @[Const.scala 11:33]
  wire  n140_valid_up; // @[Top.scala 38:22]
  wire  n140_valid_down; // @[Top.scala 38:22]
  wire [7:0] n140_I0_0; // @[Top.scala 38:22]
  wire [7:0] n140_O_0_t0b; // @[Top.scala 38:22]
  wire  n151_clk; // @[Top.scala 42:22]
  wire  n151_reset; // @[Top.scala 42:22]
  wire  n151_valid_up; // @[Top.scala 42:22]
  wire  n151_valid_down; // @[Top.scala 42:22]
  wire [7:0] n151_I_0_t0b; // @[Top.scala 42:22]
  wire [7:0] n151_O_0; // @[Top.scala 42:22]
  InitialDelayCounter InitialDelayCounter ( // @[Const.scala 11:33]
    .clk(InitialDelayCounter_clk),
    .reset(InitialDelayCounter_reset),
    .valid_down(InitialDelayCounter_valid_down)
  );
  Map2T_8 n103 ( // @[Top.scala 18:22]
    .valid_up(n103_valid_up),
    .valid_down(n103_valid_down),
    .I0_0_0(n103_I0_0_0),
    .I0_0_1(n103_I0_0_1),
    .I0_0_2(n103_I0_0_2),
    .I0_1_0(n103_I0_1_0),
    .I0_1_1(n103_I0_1_1),
    .I0_1_2(n103_I0_1_2),
    .I0_2_0(n103_I0_2_0),
    .I0_2_1(n103_I0_2_1),
    .I0_2_2(n103_I0_2_2),
    .O_0_0_t0b(n103_O_0_0_t0b),
    .O_0_0_t1b(n103_O_0_0_t1b),
    .O_0_1_t0b(n103_O_0_1_t0b),
    .O_0_1_t1b(n103_O_0_1_t1b),
    .O_0_2_t0b(n103_O_0_2_t0b),
    .O_0_2_t1b(n103_O_0_2_t1b),
    .O_1_0_t0b(n103_O_1_0_t0b),
    .O_1_0_t1b(n103_O_1_0_t1b),
    .O_1_1_t0b(n103_O_1_1_t0b),
    .O_1_1_t1b(n103_O_1_1_t1b),
    .O_1_2_t0b(n103_O_1_2_t0b),
    .O_1_2_t1b(n103_O_1_2_t1b),
    .O_2_0_t0b(n103_O_2_0_t0b),
    .O_2_0_t1b(n103_O_2_0_t1b),
    .O_2_1_t0b(n103_O_2_1_t0b),
    .O_2_1_t1b(n103_O_2_1_t1b),
    .O_2_2_t0b(n103_O_2_2_t0b),
    .O_2_2_t1b(n103_O_2_2_t1b)
  );
  MapT_7 n119 ( // @[Top.scala 22:22]
    .clk(n119_clk),
    .reset(n119_reset),
    .valid_up(n119_valid_up),
    .valid_down(n119_valid_down),
    .I_0_0_t0b(n119_I_0_0_t0b),
    .I_0_0_t1b(n119_I_0_0_t1b),
    .I_0_1_t0b(n119_I_0_1_t0b),
    .I_0_1_t1b(n119_I_0_1_t1b),
    .I_0_2_t0b(n119_I_0_2_t0b),
    .I_0_2_t1b(n119_I_0_2_t1b),
    .I_1_0_t0b(n119_I_1_0_t0b),
    .I_1_0_t1b(n119_I_1_0_t1b),
    .I_1_1_t0b(n119_I_1_1_t0b),
    .I_1_1_t1b(n119_I_1_1_t1b),
    .I_1_2_t0b(n119_I_1_2_t0b),
    .I_1_2_t1b(n119_I_1_2_t1b),
    .I_2_0_t0b(n119_I_2_0_t0b),
    .I_2_0_t1b(n119_I_2_0_t1b),
    .I_2_1_t0b(n119_I_2_1_t0b),
    .I_2_1_t1b(n119_I_2_1_t1b),
    .I_2_2_t0b(n119_I_2_2_t0b),
    .I_2_2_t1b(n119_I_2_2_t1b),
    .O_0_0(n119_O_0_0),
    .O_0_1(n119_O_0_1),
    .O_0_2(n119_O_0_2),
    .O_1_0(n119_O_1_0),
    .O_1_1(n119_O_1_1),
    .O_1_2(n119_O_1_2),
    .O_2_0(n119_O_2_0),
    .O_2_1(n119_O_2_1),
    .O_2_2(n119_O_2_2)
  );
  MapT_8 n126 ( // @[Top.scala 25:22]
    .clk(n126_clk),
    .reset(n126_reset),
    .valid_up(n126_valid_up),
    .valid_down(n126_valid_down),
    .I_0_0(n126_I_0_0),
    .I_0_1(n126_I_0_1),
    .I_0_2(n126_I_0_2),
    .I_1_0(n126_I_1_0),
    .I_1_1(n126_I_1_1),
    .I_1_2(n126_I_1_2),
    .I_2_0(n126_I_2_0),
    .I_2_1(n126_I_2_1),
    .I_2_2(n126_I_2_2),
    .O_0_0(n126_O_0_0),
    .O_1_0(n126_O_1_0),
    .O_2_0(n126_O_2_0)
  );
  MapT_9 n133 ( // @[Top.scala 28:22]
    .clk(n133_clk),
    .reset(n133_reset),
    .valid_up(n133_valid_up),
    .valid_down(n133_valid_down),
    .I_0_0(n133_I_0_0),
    .I_1_0(n133_I_1_0),
    .I_2_0(n133_I_2_0),
    .O_0_0(n133_O_0_0)
  );
  ReduceT n136 ( // @[Top.scala 31:22]
    .clk(n136_clk),
    .reset(n136_reset),
    .valid_up(n136_valid_up),
    .valid_down(n136_valid_down),
    .I_0_0(n136_I_0_0),
    .O_0_0(n136_O_0_0)
  );
  Passthrough_2 n137 ( // @[Top.scala 34:22]
    .valid_up(n137_valid_up),
    .valid_down(n137_valid_down),
    .I_0_0(n137_I_0_0),
    .O_0(n137_O_0)
  );
  InitialDelayCounter_1 InitialDelayCounter_1 ( // @[Const.scala 11:33]
    .clk(InitialDelayCounter_1_clk),
    .reset(InitialDelayCounter_1_reset),
    .valid_down(InitialDelayCounter_1_valid_down)
  );
  Map2T_9 n140 ( // @[Top.scala 38:22]
    .valid_up(n140_valid_up),
    .valid_down(n140_valid_down),
    .I0_0(n140_I0_0),
    .O_0_t0b(n140_O_0_t0b)
  );
  MapT_10 n151 ( // @[Top.scala 42:22]
    .clk(n151_clk),
    .reset(n151_reset),
    .valid_up(n151_valid_up),
    .valid_down(n151_valid_down),
    .I_0_t0b(n151_I_0_t0b),
    .O_0(n151_O_0)
  );
  assign valid_down = n151_valid_down; // @[Top.scala 46:16]
  assign O_0 = n151_O_0; // @[Top.scala 45:7]
  assign InitialDelayCounter_clk = clk;
  assign InitialDelayCounter_reset = reset;
  assign n103_valid_up = valid_up & InitialDelayCounter_valid_down; // @[Top.scala 21:19]
  assign n103_I0_0_0 = I_0_0; // @[Top.scala 19:13]
  assign n103_I0_0_1 = I_0_1; // @[Top.scala 19:13]
  assign n103_I0_0_2 = I_0_2; // @[Top.scala 19:13]
  assign n103_I0_1_0 = I_1_0; // @[Top.scala 19:13]
  assign n103_I0_1_1 = I_1_1; // @[Top.scala 19:13]
  assign n103_I0_1_2 = I_1_2; // @[Top.scala 19:13]
  assign n103_I0_2_0 = I_2_0; // @[Top.scala 19:13]
  assign n103_I0_2_1 = I_2_1; // @[Top.scala 19:13]
  assign n103_I0_2_2 = I_2_2; // @[Top.scala 19:13]
  assign n119_clk = clk;
  assign n119_reset = reset;
  assign n119_valid_up = n103_valid_down; // @[Top.scala 24:19]
  assign n119_I_0_0_t0b = n103_O_0_0_t0b; // @[Top.scala 23:12]
  assign n119_I_0_0_t1b = n103_O_0_0_t1b; // @[Top.scala 23:12]
  assign n119_I_0_1_t0b = n103_O_0_1_t0b; // @[Top.scala 23:12]
  assign n119_I_0_1_t1b = n103_O_0_1_t1b; // @[Top.scala 23:12]
  assign n119_I_0_2_t0b = n103_O_0_2_t0b; // @[Top.scala 23:12]
  assign n119_I_0_2_t1b = n103_O_0_2_t1b; // @[Top.scala 23:12]
  assign n119_I_1_0_t0b = n103_O_1_0_t0b; // @[Top.scala 23:12]
  assign n119_I_1_0_t1b = n103_O_1_0_t1b; // @[Top.scala 23:12]
  assign n119_I_1_1_t0b = n103_O_1_1_t0b; // @[Top.scala 23:12]
  assign n119_I_1_1_t1b = n103_O_1_1_t1b; // @[Top.scala 23:12]
  assign n119_I_1_2_t0b = n103_O_1_2_t0b; // @[Top.scala 23:12]
  assign n119_I_1_2_t1b = n103_O_1_2_t1b; // @[Top.scala 23:12]
  assign n119_I_2_0_t0b = n103_O_2_0_t0b; // @[Top.scala 23:12]
  assign n119_I_2_0_t1b = n103_O_2_0_t1b; // @[Top.scala 23:12]
  assign n119_I_2_1_t0b = n103_O_2_1_t0b; // @[Top.scala 23:12]
  assign n119_I_2_1_t1b = n103_O_2_1_t1b; // @[Top.scala 23:12]
  assign n119_I_2_2_t0b = n103_O_2_2_t0b; // @[Top.scala 23:12]
  assign n119_I_2_2_t1b = n103_O_2_2_t1b; // @[Top.scala 23:12]
  assign n126_clk = clk;
  assign n126_reset = reset;
  assign n126_valid_up = n119_valid_down; // @[Top.scala 27:19]
  assign n126_I_0_0 = n119_O_0_0; // @[Top.scala 26:12]
  assign n126_I_0_1 = n119_O_0_1; // @[Top.scala 26:12]
  assign n126_I_0_2 = n119_O_0_2; // @[Top.scala 26:12]
  assign n126_I_1_0 = n119_O_1_0; // @[Top.scala 26:12]
  assign n126_I_1_1 = n119_O_1_1; // @[Top.scala 26:12]
  assign n126_I_1_2 = n119_O_1_2; // @[Top.scala 26:12]
  assign n126_I_2_0 = n119_O_2_0; // @[Top.scala 26:12]
  assign n126_I_2_1 = n119_O_2_1; // @[Top.scala 26:12]
  assign n126_I_2_2 = n119_O_2_2; // @[Top.scala 26:12]
  assign n133_clk = clk;
  assign n133_reset = reset;
  assign n133_valid_up = n126_valid_down; // @[Top.scala 30:19]
  assign n133_I_0_0 = n126_O_0_0; // @[Top.scala 29:12]
  assign n133_I_1_0 = n126_O_1_0; // @[Top.scala 29:12]
  assign n133_I_2_0 = n126_O_2_0; // @[Top.scala 29:12]
  assign n136_clk = clk;
  assign n136_reset = reset;
  assign n136_valid_up = n133_valid_down; // @[Top.scala 33:19]
  assign n136_I_0_0 = n133_O_0_0; // @[Top.scala 32:12]
  assign n137_valid_up = n136_valid_down; // @[Top.scala 36:19]
  assign n137_I_0_0 = n136_O_0_0; // @[Top.scala 35:12]
  assign InitialDelayCounter_1_clk = clk;
  assign InitialDelayCounter_1_reset = reset;
  assign n140_valid_up = n137_valid_down & InitialDelayCounter_1_valid_down; // @[Top.scala 41:19]
  assign n140_I0_0 = n137_O_0; // @[Top.scala 39:13]
  assign n151_clk = clk;
  assign n151_reset = reset;
  assign n151_valid_up = n140_valid_down; // @[Top.scala 44:19]
  assign n151_I_0_t0b = n140_O_0_t0b; // @[Top.scala 43:12]
endmodule
module MapT_11(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0_0,
  input  [7:0] I_0_1,
  input  [7:0] I_0_2,
  input  [7:0] I_1_0,
  input  [7:0] I_1_1,
  input  [7:0] I_1_2,
  input  [7:0] I_2_0,
  input  [7:0] I_2_1,
  input  [7:0] I_2_2,
  output [7:0] O_0
);
  wire  op_clk; // @[MapT.scala 8:20]
  wire  op_reset; // @[MapT.scala 8:20]
  wire  op_valid_up; // @[MapT.scala 8:20]
  wire  op_valid_down; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_0_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_1_2; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_0; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_1; // @[MapT.scala 8:20]
  wire [7:0] op_I_2_2; // @[MapT.scala 8:20]
  wire [7:0] op_O_0; // @[MapT.scala 8:20]
  Module_0 op ( // @[MapT.scala 8:20]
    .clk(op_clk),
    .reset(op_reset),
    .valid_up(op_valid_up),
    .valid_down(op_valid_down),
    .I_0_0(op_I_0_0),
    .I_0_1(op_I_0_1),
    .I_0_2(op_I_0_2),
    .I_1_0(op_I_1_0),
    .I_1_1(op_I_1_1),
    .I_1_2(op_I_1_2),
    .I_2_0(op_I_2_0),
    .I_2_1(op_I_2_1),
    .I_2_2(op_I_2_2),
    .O_0(op_O_0)
  );
  assign valid_down = op_valid_down; // @[MapT.scala 16:16]
  assign O_0 = op_O_0; // @[MapT.scala 15:7]
  assign op_clk = clk;
  assign op_reset = reset;
  assign op_valid_up = valid_up; // @[MapT.scala 13:17]
  assign op_I_0_0 = I_0_0; // @[MapT.scala 14:10]
  assign op_I_0_1 = I_0_1; // @[MapT.scala 14:10]
  assign op_I_0_2 = I_0_2; // @[MapT.scala 14:10]
  assign op_I_1_0 = I_1_0; // @[MapT.scala 14:10]
  assign op_I_1_1 = I_1_1; // @[MapT.scala 14:10]
  assign op_I_1_2 = I_1_2; // @[MapT.scala 14:10]
  assign op_I_2_0 = I_2_0; // @[MapT.scala 14:10]
  assign op_I_2_1 = I_2_1; // @[MapT.scala 14:10]
  assign op_I_2_2 = I_2_2; // @[MapT.scala 14:10]
endmodule
module Passthrough_3(
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  output [7:0] O_0
);
  assign valid_down = valid_up; // @[Passthrough.scala 18:14]
  assign O_0 = I_0; // @[Passthrough.scala 17:68]
endmodule
module main(
  input        clk,
  input        reset,
  input        valid_up,
  output       valid_down,
  input  [7:0] I_0,
  output [7:0] O_0
);
  wire  n1_clk; // @[Top.scala 52:20]
  wire  n1_reset; // @[Top.scala 52:20]
  wire  n1_valid_up; // @[Top.scala 52:20]
  wire  n1_valid_down; // @[Top.scala 52:20]
  wire [7:0] n1_I_0; // @[Top.scala 52:20]
  wire [7:0] n1_O_0; // @[Top.scala 52:20]
  wire  n2_clk; // @[Top.scala 55:20]
  wire  n2_reset; // @[Top.scala 55:20]
  wire  n2_valid_up; // @[Top.scala 55:20]
  wire  n2_valid_down; // @[Top.scala 55:20]
  wire [7:0] n2_I_0; // @[Top.scala 55:20]
  wire [7:0] n2_O_0; // @[Top.scala 55:20]
  wire  n3_clk; // @[Top.scala 58:20]
  wire  n3_reset; // @[Top.scala 58:20]
  wire  n3_valid_up; // @[Top.scala 58:20]
  wire  n3_valid_down; // @[Top.scala 58:20]
  wire [7:0] n3_I_0; // @[Top.scala 58:20]
  wire [7:0] n3_O_0; // @[Top.scala 58:20]
  wire  n4_clk; // @[Top.scala 61:20]
  wire  n4_valid_up; // @[Top.scala 61:20]
  wire  n4_valid_down; // @[Top.scala 61:20]
  wire [7:0] n4_I_0; // @[Top.scala 61:20]
  wire [7:0] n4_O_0; // @[Top.scala 61:20]
  wire  n5_clk; // @[Top.scala 64:20]
  wire  n5_valid_up; // @[Top.scala 64:20]
  wire  n5_valid_down; // @[Top.scala 64:20]
  wire [7:0] n5_I_0; // @[Top.scala 64:20]
  wire [7:0] n5_O_0; // @[Top.scala 64:20]
  wire  n6_valid_up; // @[Top.scala 67:20]
  wire  n6_valid_down; // @[Top.scala 67:20]
  wire [7:0] n6_I0_0; // @[Top.scala 67:20]
  wire [7:0] n6_I1_0; // @[Top.scala 67:20]
  wire [7:0] n6_O_0_0; // @[Top.scala 67:20]
  wire [7:0] n6_O_0_1; // @[Top.scala 67:20]
  wire  n13_valid_up; // @[Top.scala 71:21]
  wire  n13_valid_down; // @[Top.scala 71:21]
  wire [7:0] n13_I0_0_0; // @[Top.scala 71:21]
  wire [7:0] n13_I0_0_1; // @[Top.scala 71:21]
  wire [7:0] n13_I1_0; // @[Top.scala 71:21]
  wire [7:0] n13_O_0_0; // @[Top.scala 71:21]
  wire [7:0] n13_O_0_1; // @[Top.scala 71:21]
  wire [7:0] n13_O_0_2; // @[Top.scala 71:21]
  wire  n22_valid_up; // @[Top.scala 75:21]
  wire  n22_valid_down; // @[Top.scala 75:21]
  wire [7:0] n22_I_0_0; // @[Top.scala 75:21]
  wire [7:0] n22_I_0_1; // @[Top.scala 75:21]
  wire [7:0] n22_I_0_2; // @[Top.scala 75:21]
  wire [7:0] n22_O_0_0_0; // @[Top.scala 75:21]
  wire [7:0] n22_O_0_0_1; // @[Top.scala 75:21]
  wire [7:0] n22_O_0_0_2; // @[Top.scala 75:21]
  wire  n29_valid_up; // @[Top.scala 78:21]
  wire  n29_valid_down; // @[Top.scala 78:21]
  wire [7:0] n29_I_0_0_0; // @[Top.scala 78:21]
  wire [7:0] n29_I_0_0_1; // @[Top.scala 78:21]
  wire [7:0] n29_I_0_0_2; // @[Top.scala 78:21]
  wire [7:0] n29_O_0_0; // @[Top.scala 78:21]
  wire [7:0] n29_O_0_1; // @[Top.scala 78:21]
  wire [7:0] n29_O_0_2; // @[Top.scala 78:21]
  wire  n30_clk; // @[Top.scala 81:21]
  wire  n30_valid_up; // @[Top.scala 81:21]
  wire  n30_valid_down; // @[Top.scala 81:21]
  wire [7:0] n30_I_0; // @[Top.scala 81:21]
  wire [7:0] n30_O_0; // @[Top.scala 81:21]
  wire  n31_clk; // @[Top.scala 84:21]
  wire  n31_valid_up; // @[Top.scala 84:21]
  wire  n31_valid_down; // @[Top.scala 84:21]
  wire [7:0] n31_I_0; // @[Top.scala 84:21]
  wire [7:0] n31_O_0; // @[Top.scala 84:21]
  wire  n32_valid_up; // @[Top.scala 87:21]
  wire  n32_valid_down; // @[Top.scala 87:21]
  wire [7:0] n32_I0_0; // @[Top.scala 87:21]
  wire [7:0] n32_I1_0; // @[Top.scala 87:21]
  wire [7:0] n32_O_0_0; // @[Top.scala 87:21]
  wire [7:0] n32_O_0_1; // @[Top.scala 87:21]
  wire  n39_valid_up; // @[Top.scala 91:21]
  wire  n39_valid_down; // @[Top.scala 91:21]
  wire [7:0] n39_I0_0_0; // @[Top.scala 91:21]
  wire [7:0] n39_I0_0_1; // @[Top.scala 91:21]
  wire [7:0] n39_I1_0; // @[Top.scala 91:21]
  wire [7:0] n39_O_0_0; // @[Top.scala 91:21]
  wire [7:0] n39_O_0_1; // @[Top.scala 91:21]
  wire [7:0] n39_O_0_2; // @[Top.scala 91:21]
  wire  n48_valid_up; // @[Top.scala 95:21]
  wire  n48_valid_down; // @[Top.scala 95:21]
  wire [7:0] n48_I_0_0; // @[Top.scala 95:21]
  wire [7:0] n48_I_0_1; // @[Top.scala 95:21]
  wire [7:0] n48_I_0_2; // @[Top.scala 95:21]
  wire [7:0] n48_O_0_0_0; // @[Top.scala 95:21]
  wire [7:0] n48_O_0_0_1; // @[Top.scala 95:21]
  wire [7:0] n48_O_0_0_2; // @[Top.scala 95:21]
  wire  n55_valid_up; // @[Top.scala 98:21]
  wire  n55_valid_down; // @[Top.scala 98:21]
  wire [7:0] n55_I_0_0_0; // @[Top.scala 98:21]
  wire [7:0] n55_I_0_0_1; // @[Top.scala 98:21]
  wire [7:0] n55_I_0_0_2; // @[Top.scala 98:21]
  wire [7:0] n55_O_0_0; // @[Top.scala 98:21]
  wire [7:0] n55_O_0_1; // @[Top.scala 98:21]
  wire [7:0] n55_O_0_2; // @[Top.scala 98:21]
  wire  n56_valid_up; // @[Top.scala 101:21]
  wire  n56_valid_down; // @[Top.scala 101:21]
  wire [7:0] n56_I0_0_0; // @[Top.scala 101:21]
  wire [7:0] n56_I0_0_1; // @[Top.scala 101:21]
  wire [7:0] n56_I0_0_2; // @[Top.scala 101:21]
  wire [7:0] n56_I1_0_0; // @[Top.scala 101:21]
  wire [7:0] n56_I1_0_1; // @[Top.scala 101:21]
  wire [7:0] n56_I1_0_2; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_0_0; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_0_1; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_0_2; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_1_0; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_1_1; // @[Top.scala 101:21]
  wire [7:0] n56_O_0_1_2; // @[Top.scala 101:21]
  wire  n63_clk; // @[Top.scala 105:21]
  wire  n63_valid_up; // @[Top.scala 105:21]
  wire  n63_valid_down; // @[Top.scala 105:21]
  wire [7:0] n63_I_0; // @[Top.scala 105:21]
  wire [7:0] n63_O_0; // @[Top.scala 105:21]
  wire  n64_clk; // @[Top.scala 108:21]
  wire  n64_valid_up; // @[Top.scala 108:21]
  wire  n64_valid_down; // @[Top.scala 108:21]
  wire [7:0] n64_I_0; // @[Top.scala 108:21]
  wire [7:0] n64_O_0; // @[Top.scala 108:21]
  wire  n65_valid_up; // @[Top.scala 111:21]
  wire  n65_valid_down; // @[Top.scala 111:21]
  wire [7:0] n65_I0_0; // @[Top.scala 111:21]
  wire [7:0] n65_I1_0; // @[Top.scala 111:21]
  wire [7:0] n65_O_0_0; // @[Top.scala 111:21]
  wire [7:0] n65_O_0_1; // @[Top.scala 111:21]
  wire  n72_valid_up; // @[Top.scala 115:21]
  wire  n72_valid_down; // @[Top.scala 115:21]
  wire [7:0] n72_I0_0_0; // @[Top.scala 115:21]
  wire [7:0] n72_I0_0_1; // @[Top.scala 115:21]
  wire [7:0] n72_I1_0; // @[Top.scala 115:21]
  wire [7:0] n72_O_0_0; // @[Top.scala 115:21]
  wire [7:0] n72_O_0_1; // @[Top.scala 115:21]
  wire [7:0] n72_O_0_2; // @[Top.scala 115:21]
  wire  n81_valid_up; // @[Top.scala 119:21]
  wire  n81_valid_down; // @[Top.scala 119:21]
  wire [7:0] n81_I_0_0; // @[Top.scala 119:21]
  wire [7:0] n81_I_0_1; // @[Top.scala 119:21]
  wire [7:0] n81_I_0_2; // @[Top.scala 119:21]
  wire [7:0] n81_O_0_0_0; // @[Top.scala 119:21]
  wire [7:0] n81_O_0_0_1; // @[Top.scala 119:21]
  wire [7:0] n81_O_0_0_2; // @[Top.scala 119:21]
  wire  n88_valid_up; // @[Top.scala 122:21]
  wire  n88_valid_down; // @[Top.scala 122:21]
  wire [7:0] n88_I_0_0_0; // @[Top.scala 122:21]
  wire [7:0] n88_I_0_0_1; // @[Top.scala 122:21]
  wire [7:0] n88_I_0_0_2; // @[Top.scala 122:21]
  wire [7:0] n88_O_0_0; // @[Top.scala 122:21]
  wire [7:0] n88_O_0_1; // @[Top.scala 122:21]
  wire [7:0] n88_O_0_2; // @[Top.scala 122:21]
  wire  n89_valid_up; // @[Top.scala 125:21]
  wire  n89_valid_down; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_0_0; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_0_1; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_0_2; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_1_0; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_1_1; // @[Top.scala 125:21]
  wire [7:0] n89_I0_0_1_2; // @[Top.scala 125:21]
  wire [7:0] n89_I1_0_0; // @[Top.scala 125:21]
  wire [7:0] n89_I1_0_1; // @[Top.scala 125:21]
  wire [7:0] n89_I1_0_2; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_0_0; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_0_1; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_0_2; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_1_0; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_1_1; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_1_2; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_2_0; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_2_1; // @[Top.scala 125:21]
  wire [7:0] n89_O_0_2_2; // @[Top.scala 125:21]
  wire  n96_valid_up; // @[Top.scala 129:21]
  wire  n96_valid_down; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_0_0; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_0_1; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_0_2; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_1_0; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_1_1; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_1_2; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_2_0; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_2_1; // @[Top.scala 129:21]
  wire [7:0] n96_I_0_2_2; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_0_0; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_0_1; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_0_2; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_1_0; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_1_1; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_1_2; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_2_0; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_2_1; // @[Top.scala 129:21]
  wire [7:0] n96_O_0_2_2; // @[Top.scala 129:21]
  wire  n99_valid_up; // @[Top.scala 132:21]
  wire  n99_valid_down; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_0_0; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_0_1; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_0_2; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_1_0; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_1_1; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_1_2; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_2_0; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_2_1; // @[Top.scala 132:21]
  wire [7:0] n99_I_0_2_2; // @[Top.scala 132:21]
  wire [7:0] n99_O_0_0; // @[Top.scala 132:21]
  wire [7:0] n99_O_0_1; // @[Top.scala 132:21]
  wire [7:0] n99_O_0_2; // @[Top.scala 132:21]
  wire [7:0] n99_O_1_0; // @[Top.scala 132:21]
  wire [7:0] n99_O_1_1; // @[Top.scala 132:21]
  wire [7:0] n99_O_1_2; // @[Top.scala 132:21]
  wire [7:0] n99_O_2_0; // @[Top.scala 132:21]
  wire [7:0] n99_O_2_1; // @[Top.scala 132:21]
  wire [7:0] n99_O_2_2; // @[Top.scala 132:21]
  wire  n152_clk; // @[Top.scala 135:22]
  wire  n152_reset; // @[Top.scala 135:22]
  wire  n152_valid_up; // @[Top.scala 135:22]
  wire  n152_valid_down; // @[Top.scala 135:22]
  wire [7:0] n152_I_0_0; // @[Top.scala 135:22]
  wire [7:0] n152_I_0_1; // @[Top.scala 135:22]
  wire [7:0] n152_I_0_2; // @[Top.scala 135:22]
  wire [7:0] n152_I_1_0; // @[Top.scala 135:22]
  wire [7:0] n152_I_1_1; // @[Top.scala 135:22]
  wire [7:0] n152_I_1_2; // @[Top.scala 135:22]
  wire [7:0] n152_I_2_0; // @[Top.scala 135:22]
  wire [7:0] n152_I_2_1; // @[Top.scala 135:22]
  wire [7:0] n152_I_2_2; // @[Top.scala 135:22]
  wire [7:0] n152_O_0; // @[Top.scala 135:22]
  wire  n153_valid_up; // @[Top.scala 138:22]
  wire  n153_valid_down; // @[Top.scala 138:22]
  wire [7:0] n153_I_0; // @[Top.scala 138:22]
  wire [7:0] n153_O_0; // @[Top.scala 138:22]
  wire  n154_valid_up; // @[Top.scala 141:22]
  wire  n154_valid_down; // @[Top.scala 141:22]
  wire [7:0] n154_I_0; // @[Top.scala 141:22]
  wire [7:0] n154_O_0; // @[Top.scala 141:22]
  wire  n155_clk; // @[Top.scala 144:22]
  wire  n155_reset; // @[Top.scala 144:22]
  wire  n155_valid_up; // @[Top.scala 144:22]
  wire  n155_valid_down; // @[Top.scala 144:22]
  wire [7:0] n155_I_0; // @[Top.scala 144:22]
  wire [7:0] n155_O_0; // @[Top.scala 144:22]
  wire  n156_clk; // @[Top.scala 147:22]
  wire  n156_reset; // @[Top.scala 147:22]
  wire  n156_valid_up; // @[Top.scala 147:22]
  wire  n156_valid_down; // @[Top.scala 147:22]
  wire [7:0] n156_I_0; // @[Top.scala 147:22]
  wire [7:0] n156_O_0; // @[Top.scala 147:22]
  wire  n157_clk; // @[Top.scala 150:22]
  wire  n157_reset; // @[Top.scala 150:22]
  wire  n157_valid_up; // @[Top.scala 150:22]
  wire  n157_valid_down; // @[Top.scala 150:22]
  wire [7:0] n157_I_0; // @[Top.scala 150:22]
  wire [7:0] n157_O_0; // @[Top.scala 150:22]
  FIFO n1 ( // @[Top.scala 52:20]
    .clk(n1_clk),
    .reset(n1_reset),
    .valid_up(n1_valid_up),
    .valid_down(n1_valid_down),
    .I_0(n1_I_0),
    .O_0(n1_O_0)
  );
  ShiftT n2 ( // @[Top.scala 55:20]
    .clk(n2_clk),
    .reset(n2_reset),
    .valid_up(n2_valid_up),
    .valid_down(n2_valid_down),
    .I_0(n2_I_0),
    .O_0(n2_O_0)
  );
  ShiftT n3 ( // @[Top.scala 58:20]
    .clk(n3_clk),
    .reset(n3_reset),
    .valid_up(n3_valid_up),
    .valid_down(n3_valid_down),
    .I_0(n3_I_0),
    .O_0(n3_O_0)
  );
  ShiftT_2 n4 ( // @[Top.scala 61:20]
    .clk(n4_clk),
    .valid_up(n4_valid_up),
    .valid_down(n4_valid_down),
    .I_0(n4_I_0),
    .O_0(n4_O_0)
  );
  ShiftT_2 n5 ( // @[Top.scala 64:20]
    .clk(n5_clk),
    .valid_up(n5_valid_up),
    .valid_down(n5_valid_down),
    .I_0(n5_I_0),
    .O_0(n5_O_0)
  );
  Map2T n6 ( // @[Top.scala 67:20]
    .valid_up(n6_valid_up),
    .valid_down(n6_valid_down),
    .I0_0(n6_I0_0),
    .I1_0(n6_I1_0),
    .O_0_0(n6_O_0_0),
    .O_0_1(n6_O_0_1)
  );
  Map2T_1 n13 ( // @[Top.scala 71:21]
    .valid_up(n13_valid_up),
    .valid_down(n13_valid_down),
    .I0_0_0(n13_I0_0_0),
    .I0_0_1(n13_I0_0_1),
    .I1_0(n13_I1_0),
    .O_0_0(n13_O_0_0),
    .O_0_1(n13_O_0_1),
    .O_0_2(n13_O_0_2)
  );
  MapT n22 ( // @[Top.scala 75:21]
    .valid_up(n22_valid_up),
    .valid_down(n22_valid_down),
    .I_0_0(n22_I_0_0),
    .I_0_1(n22_I_0_1),
    .I_0_2(n22_I_0_2),
    .O_0_0_0(n22_O_0_0_0),
    .O_0_0_1(n22_O_0_0_1),
    .O_0_0_2(n22_O_0_0_2)
  );
  MapT_1 n29 ( // @[Top.scala 78:21]
    .valid_up(n29_valid_up),
    .valid_down(n29_valid_down),
    .I_0_0_0(n29_I_0_0_0),
    .I_0_0_1(n29_I_0_0_1),
    .I_0_0_2(n29_I_0_0_2),
    .O_0_0(n29_O_0_0),
    .O_0_1(n29_O_0_1),
    .O_0_2(n29_O_0_2)
  );
  ShiftT_2 n30 ( // @[Top.scala 81:21]
    .clk(n30_clk),
    .valid_up(n30_valid_up),
    .valid_down(n30_valid_down),
    .I_0(n30_I_0),
    .O_0(n30_O_0)
  );
  ShiftT_2 n31 ( // @[Top.scala 84:21]
    .clk(n31_clk),
    .valid_up(n31_valid_up),
    .valid_down(n31_valid_down),
    .I_0(n31_I_0),
    .O_0(n31_O_0)
  );
  Map2T n32 ( // @[Top.scala 87:21]
    .valid_up(n32_valid_up),
    .valid_down(n32_valid_down),
    .I0_0(n32_I0_0),
    .I1_0(n32_I1_0),
    .O_0_0(n32_O_0_0),
    .O_0_1(n32_O_0_1)
  );
  Map2T_1 n39 ( // @[Top.scala 91:21]
    .valid_up(n39_valid_up),
    .valid_down(n39_valid_down),
    .I0_0_0(n39_I0_0_0),
    .I0_0_1(n39_I0_0_1),
    .I1_0(n39_I1_0),
    .O_0_0(n39_O_0_0),
    .O_0_1(n39_O_0_1),
    .O_0_2(n39_O_0_2)
  );
  MapT n48 ( // @[Top.scala 95:21]
    .valid_up(n48_valid_up),
    .valid_down(n48_valid_down),
    .I_0_0(n48_I_0_0),
    .I_0_1(n48_I_0_1),
    .I_0_2(n48_I_0_2),
    .O_0_0_0(n48_O_0_0_0),
    .O_0_0_1(n48_O_0_0_1),
    .O_0_0_2(n48_O_0_0_2)
  );
  MapT_1 n55 ( // @[Top.scala 98:21]
    .valid_up(n55_valid_up),
    .valid_down(n55_valid_down),
    .I_0_0_0(n55_I_0_0_0),
    .I_0_0_1(n55_I_0_0_1),
    .I_0_0_2(n55_I_0_0_2),
    .O_0_0(n55_O_0_0),
    .O_0_1(n55_O_0_1),
    .O_0_2(n55_O_0_2)
  );
  Map2T_4 n56 ( // @[Top.scala 101:21]
    .valid_up(n56_valid_up),
    .valid_down(n56_valid_down),
    .I0_0_0(n56_I0_0_0),
    .I0_0_1(n56_I0_0_1),
    .I0_0_2(n56_I0_0_2),
    .I1_0_0(n56_I1_0_0),
    .I1_0_1(n56_I1_0_1),
    .I1_0_2(n56_I1_0_2),
    .O_0_0_0(n56_O_0_0_0),
    .O_0_0_1(n56_O_0_0_1),
    .O_0_0_2(n56_O_0_0_2),
    .O_0_1_0(n56_O_0_1_0),
    .O_0_1_1(n56_O_0_1_1),
    .O_0_1_2(n56_O_0_1_2)
  );
  ShiftT_2 n63 ( // @[Top.scala 105:21]
    .clk(n63_clk),
    .valid_up(n63_valid_up),
    .valid_down(n63_valid_down),
    .I_0(n63_I_0),
    .O_0(n63_O_0)
  );
  ShiftT_2 n64 ( // @[Top.scala 108:21]
    .clk(n64_clk),
    .valid_up(n64_valid_up),
    .valid_down(n64_valid_down),
    .I_0(n64_I_0),
    .O_0(n64_O_0)
  );
  Map2T n65 ( // @[Top.scala 111:21]
    .valid_up(n65_valid_up),
    .valid_down(n65_valid_down),
    .I0_0(n65_I0_0),
    .I1_0(n65_I1_0),
    .O_0_0(n65_O_0_0),
    .O_0_1(n65_O_0_1)
  );
  Map2T_1 n72 ( // @[Top.scala 115:21]
    .valid_up(n72_valid_up),
    .valid_down(n72_valid_down),
    .I0_0_0(n72_I0_0_0),
    .I0_0_1(n72_I0_0_1),
    .I1_0(n72_I1_0),
    .O_0_0(n72_O_0_0),
    .O_0_1(n72_O_0_1),
    .O_0_2(n72_O_0_2)
  );
  MapT n81 ( // @[Top.scala 119:21]
    .valid_up(n81_valid_up),
    .valid_down(n81_valid_down),
    .I_0_0(n81_I_0_0),
    .I_0_1(n81_I_0_1),
    .I_0_2(n81_I_0_2),
    .O_0_0_0(n81_O_0_0_0),
    .O_0_0_1(n81_O_0_0_1),
    .O_0_0_2(n81_O_0_0_2)
  );
  MapT_1 n88 ( // @[Top.scala 122:21]
    .valid_up(n88_valid_up),
    .valid_down(n88_valid_down),
    .I_0_0_0(n88_I_0_0_0),
    .I_0_0_1(n88_I_0_0_1),
    .I_0_0_2(n88_I_0_0_2),
    .O_0_0(n88_O_0_0),
    .O_0_1(n88_O_0_1),
    .O_0_2(n88_O_0_2)
  );
  Map2T_7 n89 ( // @[Top.scala 125:21]
    .valid_up(n89_valid_up),
    .valid_down(n89_valid_down),
    .I0_0_0_0(n89_I0_0_0_0),
    .I0_0_0_1(n89_I0_0_0_1),
    .I0_0_0_2(n89_I0_0_0_2),
    .I0_0_1_0(n89_I0_0_1_0),
    .I0_0_1_1(n89_I0_0_1_1),
    .I0_0_1_2(n89_I0_0_1_2),
    .I1_0_0(n89_I1_0_0),
    .I1_0_1(n89_I1_0_1),
    .I1_0_2(n89_I1_0_2),
    .O_0_0_0(n89_O_0_0_0),
    .O_0_0_1(n89_O_0_0_1),
    .O_0_0_2(n89_O_0_0_2),
    .O_0_1_0(n89_O_0_1_0),
    .O_0_1_1(n89_O_0_1_1),
    .O_0_1_2(n89_O_0_1_2),
    .O_0_2_0(n89_O_0_2_0),
    .O_0_2_1(n89_O_0_2_1),
    .O_0_2_2(n89_O_0_2_2)
  );
  Passthrough n96 ( // @[Top.scala 129:21]
    .valid_up(n96_valid_up),
    .valid_down(n96_valid_down),
    .I_0_0_0(n96_I_0_0_0),
    .I_0_0_1(n96_I_0_0_1),
    .I_0_0_2(n96_I_0_0_2),
    .I_0_1_0(n96_I_0_1_0),
    .I_0_1_1(n96_I_0_1_1),
    .I_0_1_2(n96_I_0_1_2),
    .I_0_2_0(n96_I_0_2_0),
    .I_0_2_1(n96_I_0_2_1),
    .I_0_2_2(n96_I_0_2_2),
    .O_0_0_0(n96_O_0_0_0),
    .O_0_0_1(n96_O_0_0_1),
    .O_0_0_2(n96_O_0_0_2),
    .O_0_1_0(n96_O_0_1_0),
    .O_0_1_1(n96_O_0_1_1),
    .O_0_1_2(n96_O_0_1_2),
    .O_0_2_0(n96_O_0_2_0),
    .O_0_2_1(n96_O_0_2_1),
    .O_0_2_2(n96_O_0_2_2)
  );
  MapT_6 n99 ( // @[Top.scala 132:21]
    .valid_up(n99_valid_up),
    .valid_down(n99_valid_down),
    .I_0_0_0(n99_I_0_0_0),
    .I_0_0_1(n99_I_0_0_1),
    .I_0_0_2(n99_I_0_0_2),
    .I_0_1_0(n99_I_0_1_0),
    .I_0_1_1(n99_I_0_1_1),
    .I_0_1_2(n99_I_0_1_2),
    .I_0_2_0(n99_I_0_2_0),
    .I_0_2_1(n99_I_0_2_1),
    .I_0_2_2(n99_I_0_2_2),
    .O_0_0(n99_O_0_0),
    .O_0_1(n99_O_0_1),
    .O_0_2(n99_O_0_2),
    .O_1_0(n99_O_1_0),
    .O_1_1(n99_O_1_1),
    .O_1_2(n99_O_1_2),
    .O_2_0(n99_O_2_0),
    .O_2_1(n99_O_2_1),
    .O_2_2(n99_O_2_2)
  );
  MapT_11 n152 ( // @[Top.scala 135:22]
    .clk(n152_clk),
    .reset(n152_reset),
    .valid_up(n152_valid_up),
    .valid_down(n152_valid_down),
    .I_0_0(n152_I_0_0),
    .I_0_1(n152_I_0_1),
    .I_0_2(n152_I_0_2),
    .I_1_0(n152_I_1_0),
    .I_1_1(n152_I_1_1),
    .I_1_2(n152_I_1_2),
    .I_2_0(n152_I_2_0),
    .I_2_1(n152_I_2_1),
    .I_2_2(n152_I_2_2),
    .O_0(n152_O_0)
  );
  Passthrough_3 n153 ( // @[Top.scala 138:22]
    .valid_up(n153_valid_up),
    .valid_down(n153_valid_down),
    .I_0(n153_I_0),
    .O_0(n153_O_0)
  );
  Passthrough_3 n154 ( // @[Top.scala 141:22]
    .valid_up(n154_valid_up),
    .valid_down(n154_valid_down),
    .I_0(n154_I_0),
    .O_0(n154_O_0)
  );
  FIFO n155 ( // @[Top.scala 144:22]
    .clk(n155_clk),
    .reset(n155_reset),
    .valid_up(n155_valid_up),
    .valid_down(n155_valid_down),
    .I_0(n155_I_0),
    .O_0(n155_O_0)
  );
  FIFO n156 ( // @[Top.scala 147:22]
    .clk(n156_clk),
    .reset(n156_reset),
    .valid_up(n156_valid_up),
    .valid_down(n156_valid_down),
    .I_0(n156_I_0),
    .O_0(n156_O_0)
  );
  FIFO n157 ( // @[Top.scala 150:22]
    .clk(n157_clk),
    .reset(n157_reset),
    .valid_up(n157_valid_up),
    .valid_down(n157_valid_down),
    .I_0(n157_I_0),
    .O_0(n157_O_0)
  );
  assign valid_down = n157_valid_down; // @[Top.scala 154:16]
  assign O_0 = n157_O_0; // @[Top.scala 153:7]
  assign n1_clk = clk;
  assign n1_reset = reset;
  assign n1_valid_up = valid_up; // @[Top.scala 54:17]
  assign n1_I_0 = I_0; // @[Top.scala 53:10]
  assign n2_clk = clk;
  assign n2_reset = reset;
  assign n2_valid_up = n1_valid_down; // @[Top.scala 57:17]
  assign n2_I_0 = n1_O_0; // @[Top.scala 56:10]
  assign n3_clk = clk;
  assign n3_reset = reset;
  assign n3_valid_up = n2_valid_down; // @[Top.scala 60:17]
  assign n3_I_0 = n2_O_0; // @[Top.scala 59:10]
  assign n4_clk = clk;
  assign n4_valid_up = n3_valid_down; // @[Top.scala 63:17]
  assign n4_I_0 = n3_O_0; // @[Top.scala 62:10]
  assign n5_clk = clk;
  assign n5_valid_up = n4_valid_down; // @[Top.scala 66:17]
  assign n5_I_0 = n4_O_0; // @[Top.scala 65:10]
  assign n6_valid_up = n5_valid_down & n4_valid_down; // @[Top.scala 70:17]
  assign n6_I0_0 = n5_O_0; // @[Top.scala 68:11]
  assign n6_I1_0 = n4_O_0; // @[Top.scala 69:11]
  assign n13_valid_up = n6_valid_down & n3_valid_down; // @[Top.scala 74:18]
  assign n13_I0_0_0 = n6_O_0_0; // @[Top.scala 72:12]
  assign n13_I0_0_1 = n6_O_0_1; // @[Top.scala 72:12]
  assign n13_I1_0 = n3_O_0; // @[Top.scala 73:12]
  assign n22_valid_up = n13_valid_down; // @[Top.scala 77:18]
  assign n22_I_0_0 = n13_O_0_0; // @[Top.scala 76:11]
  assign n22_I_0_1 = n13_O_0_1; // @[Top.scala 76:11]
  assign n22_I_0_2 = n13_O_0_2; // @[Top.scala 76:11]
  assign n29_valid_up = n22_valid_down; // @[Top.scala 80:18]
  assign n29_I_0_0_0 = n22_O_0_0_0; // @[Top.scala 79:11]
  assign n29_I_0_0_1 = n22_O_0_0_1; // @[Top.scala 79:11]
  assign n29_I_0_0_2 = n22_O_0_0_2; // @[Top.scala 79:11]
  assign n30_clk = clk;
  assign n30_valid_up = n2_valid_down; // @[Top.scala 83:18]
  assign n30_I_0 = n2_O_0; // @[Top.scala 82:11]
  assign n31_clk = clk;
  assign n31_valid_up = n30_valid_down; // @[Top.scala 86:18]
  assign n31_I_0 = n30_O_0; // @[Top.scala 85:11]
  assign n32_valid_up = n31_valid_down & n30_valid_down; // @[Top.scala 90:18]
  assign n32_I0_0 = n31_O_0; // @[Top.scala 88:12]
  assign n32_I1_0 = n30_O_0; // @[Top.scala 89:12]
  assign n39_valid_up = n32_valid_down & n2_valid_down; // @[Top.scala 94:18]
  assign n39_I0_0_0 = n32_O_0_0; // @[Top.scala 92:12]
  assign n39_I0_0_1 = n32_O_0_1; // @[Top.scala 92:12]
  assign n39_I1_0 = n2_O_0; // @[Top.scala 93:12]
  assign n48_valid_up = n39_valid_down; // @[Top.scala 97:18]
  assign n48_I_0_0 = n39_O_0_0; // @[Top.scala 96:11]
  assign n48_I_0_1 = n39_O_0_1; // @[Top.scala 96:11]
  assign n48_I_0_2 = n39_O_0_2; // @[Top.scala 96:11]
  assign n55_valid_up = n48_valid_down; // @[Top.scala 100:18]
  assign n55_I_0_0_0 = n48_O_0_0_0; // @[Top.scala 99:11]
  assign n55_I_0_0_1 = n48_O_0_0_1; // @[Top.scala 99:11]
  assign n55_I_0_0_2 = n48_O_0_0_2; // @[Top.scala 99:11]
  assign n56_valid_up = n29_valid_down & n55_valid_down; // @[Top.scala 104:18]
  assign n56_I0_0_0 = n29_O_0_0; // @[Top.scala 102:12]
  assign n56_I0_0_1 = n29_O_0_1; // @[Top.scala 102:12]
  assign n56_I0_0_2 = n29_O_0_2; // @[Top.scala 102:12]
  assign n56_I1_0_0 = n55_O_0_0; // @[Top.scala 103:12]
  assign n56_I1_0_1 = n55_O_0_1; // @[Top.scala 103:12]
  assign n56_I1_0_2 = n55_O_0_2; // @[Top.scala 103:12]
  assign n63_clk = clk;
  assign n63_valid_up = n1_valid_down; // @[Top.scala 107:18]
  assign n63_I_0 = n1_O_0; // @[Top.scala 106:11]
  assign n64_clk = clk;
  assign n64_valid_up = n63_valid_down; // @[Top.scala 110:18]
  assign n64_I_0 = n63_O_0; // @[Top.scala 109:11]
  assign n65_valid_up = n64_valid_down & n63_valid_down; // @[Top.scala 114:18]
  assign n65_I0_0 = n64_O_0; // @[Top.scala 112:12]
  assign n65_I1_0 = n63_O_0; // @[Top.scala 113:12]
  assign n72_valid_up = n65_valid_down & n1_valid_down; // @[Top.scala 118:18]
  assign n72_I0_0_0 = n65_O_0_0; // @[Top.scala 116:12]
  assign n72_I0_0_1 = n65_O_0_1; // @[Top.scala 116:12]
  assign n72_I1_0 = n1_O_0; // @[Top.scala 117:12]
  assign n81_valid_up = n72_valid_down; // @[Top.scala 121:18]
  assign n81_I_0_0 = n72_O_0_0; // @[Top.scala 120:11]
  assign n81_I_0_1 = n72_O_0_1; // @[Top.scala 120:11]
  assign n81_I_0_2 = n72_O_0_2; // @[Top.scala 120:11]
  assign n88_valid_up = n81_valid_down; // @[Top.scala 124:18]
  assign n88_I_0_0_0 = n81_O_0_0_0; // @[Top.scala 123:11]
  assign n88_I_0_0_1 = n81_O_0_0_1; // @[Top.scala 123:11]
  assign n88_I_0_0_2 = n81_O_0_0_2; // @[Top.scala 123:11]
  assign n89_valid_up = n56_valid_down & n88_valid_down; // @[Top.scala 128:18]
  assign n89_I0_0_0_0 = n56_O_0_0_0; // @[Top.scala 126:12]
  assign n89_I0_0_0_1 = n56_O_0_0_1; // @[Top.scala 126:12]
  assign n89_I0_0_0_2 = n56_O_0_0_2; // @[Top.scala 126:12]
  assign n89_I0_0_1_0 = n56_O_0_1_0; // @[Top.scala 126:12]
  assign n89_I0_0_1_1 = n56_O_0_1_1; // @[Top.scala 126:12]
  assign n89_I0_0_1_2 = n56_O_0_1_2; // @[Top.scala 126:12]
  assign n89_I1_0_0 = n88_O_0_0; // @[Top.scala 127:12]
  assign n89_I1_0_1 = n88_O_0_1; // @[Top.scala 127:12]
  assign n89_I1_0_2 = n88_O_0_2; // @[Top.scala 127:12]
  assign n96_valid_up = n89_valid_down; // @[Top.scala 131:18]
  assign n96_I_0_0_0 = n89_O_0_0_0; // @[Top.scala 130:11]
  assign n96_I_0_0_1 = n89_O_0_0_1; // @[Top.scala 130:11]
  assign n96_I_0_0_2 = n89_O_0_0_2; // @[Top.scala 130:11]
  assign n96_I_0_1_0 = n89_O_0_1_0; // @[Top.scala 130:11]
  assign n96_I_0_1_1 = n89_O_0_1_1; // @[Top.scala 130:11]
  assign n96_I_0_1_2 = n89_O_0_1_2; // @[Top.scala 130:11]
  assign n96_I_0_2_0 = n89_O_0_2_0; // @[Top.scala 130:11]
  assign n96_I_0_2_1 = n89_O_0_2_1; // @[Top.scala 130:11]
  assign n96_I_0_2_2 = n89_O_0_2_2; // @[Top.scala 130:11]
  assign n99_valid_up = n96_valid_down; // @[Top.scala 134:18]
  assign n99_I_0_0_0 = n96_O_0_0_0; // @[Top.scala 133:11]
  assign n99_I_0_0_1 = n96_O_0_0_1; // @[Top.scala 133:11]
  assign n99_I_0_0_2 = n96_O_0_0_2; // @[Top.scala 133:11]
  assign n99_I_0_1_0 = n96_O_0_1_0; // @[Top.scala 133:11]
  assign n99_I_0_1_1 = n96_O_0_1_1; // @[Top.scala 133:11]
  assign n99_I_0_1_2 = n96_O_0_1_2; // @[Top.scala 133:11]
  assign n99_I_0_2_0 = n96_O_0_2_0; // @[Top.scala 133:11]
  assign n99_I_0_2_1 = n96_O_0_2_1; // @[Top.scala 133:11]
  assign n99_I_0_2_2 = n96_O_0_2_2; // @[Top.scala 133:11]
  assign n152_clk = clk;
  assign n152_reset = reset;
  assign n152_valid_up = n99_valid_down; // @[Top.scala 137:19]
  assign n152_I_0_0 = n99_O_0_0; // @[Top.scala 136:12]
  assign n152_I_0_1 = n99_O_0_1; // @[Top.scala 136:12]
  assign n152_I_0_2 = n99_O_0_2; // @[Top.scala 136:12]
  assign n152_I_1_0 = n99_O_1_0; // @[Top.scala 136:12]
  assign n152_I_1_1 = n99_O_1_1; // @[Top.scala 136:12]
  assign n152_I_1_2 = n99_O_1_2; // @[Top.scala 136:12]
  assign n152_I_2_0 = n99_O_2_0; // @[Top.scala 136:12]
  assign n152_I_2_1 = n99_O_2_1; // @[Top.scala 136:12]
  assign n152_I_2_2 = n99_O_2_2; // @[Top.scala 136:12]
  assign n153_valid_up = n152_valid_down; // @[Top.scala 140:19]
  assign n153_I_0 = n152_O_0; // @[Top.scala 139:12]
  assign n154_valid_up = n153_valid_down; // @[Top.scala 143:19]
  assign n154_I_0 = n153_O_0; // @[Top.scala 142:12]
  assign n155_clk = clk;
  assign n155_reset = reset;
  assign n155_valid_up = n154_valid_down; // @[Top.scala 146:19]
  assign n155_I_0 = n154_O_0; // @[Top.scala 145:12]
  assign n156_clk = clk;
  assign n156_reset = reset;
  assign n156_valid_up = n155_valid_down; // @[Top.scala 149:19]
  assign n156_I_0 = n155_O_0; // @[Top.scala 148:12]
  assign n157_clk = clk;
  assign n157_reset = reset;
  assign n157_valid_up = n156_valid_down; // @[Top.scala 152:19]
  assign n157_I_0 = n156_O_0; // @[Top.scala 151:12]
endmodule
module BlackBoxMulUInt8(
	               input wire [7:0] I0,
	               input wire [7:0] I1,
	               output wire [15:0] O,
	               input wire       clk
	               );
`ifdef VERILATOR
   reg [15:0] reg0;
   reg [15:0] reg1;
   reg [15:0] reg2;
   always @( posedge clk ) begin
      reg0 <= I0 * I1;
      reg1 <= reg0;
      reg2 <= reg1;
   end
   assign O = reg2;
`endif
`ifndef VERILATOR
   mul_uint8 mul_uint8 (
                   .A(I0),
                   .B(I1),
                   .P(O),
                   .CLK(clk)
                   );
`endif
endmodule