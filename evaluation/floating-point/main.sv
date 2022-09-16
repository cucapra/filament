/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

module std_const #(
    parameter WIDTH = 32,
    parameter VALUE = 0
) (
   output logic [WIDTH - 1:0] out
);
  assign out = VALUE;
endmodule

module std_wire #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH - 1:0] in,
  output logic [WIDTH - 1:0] out
);
  assign out = in;
endmodule

module std_slice #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire                   logic [ IN_WIDTH-1:0] in,
   output logic [OUT_WIDTH-1:0] out
);
  assign out = in[OUT_WIDTH-1:0];

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH < OUT_WIDTH)
        $error(
          "std_slice: Input width less than output width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_pad #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire logic [IN_WIDTH-1:0]  in,
   output logic     [OUT_WIDTH-1:0] out
);
  localparam EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = { {EXTEND {1'b0}}, in};

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH > OUT_WIDTH)
        $error(
          "std_pad: Output width less than input width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_not #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

module std_and #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module std_or #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module std_xor #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module std_sub #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_gt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left > right;
endmodule

module std_lt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left < right;
endmodule

module std_eq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left == right;
endmodule

module std_neq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left != right;
endmodule

module std_ge #(
    parameter WIDTH = 32
) (
    input wire   logic [WIDTH-1:0] left,
    input wire   logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left >= right;
endmodule

module std_le #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left <= right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left << right;
endmodule

module std_rsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left >> right;
endmodule

/// this primitive is intended to be used
/// for lowering purposes (not in source programs)
module std_mux #(
    parameter WIDTH = 32
) (
   input wire               logic cond,
   input wire               logic [WIDTH-1:0] tru,
   input wire               logic [WIDTH-1:0] fal,
   output logic [WIDTH-1:0] out
);
  assign out = cond ? tru : fal;
endmodule

/// Memories
module std_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in,
   input wire                 write_en,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out,
   output logic               done
);

  always_ff @(posedge clk) begin
    if (reset) begin
       out <= 0;
       done <= 0;
    end else if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

module std_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   input wire                logic [IDX_SIZE-1:0] addr0,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= SIZE)
        $error(
          "std_mem_d1: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "SIZE: %0d", SIZE
        );
    end
  `endif
endmodule

module std_mem_d2 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
    end
  `endif
endmodule

module std_mem_d3 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
    end
  `endif
endmodule

module std_mem_d4 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D3_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4,
    parameter D3_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [D3_IDX_SIZE-1:0] addr3,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2][addr3] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
      if (addr3 >= D3_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr3: %0d\n", addr3,
          "D3_SIZE: %0d", D3_SIZE
        );
    end
  `endif
endmodule

`default_nettype wire

module FP_Mult_NoPipe (
		input _go, // unused
    input [31:0] a,
    input [31:0] b,
    output exception,
    output overflow,
    output underflow,
    output [31:0] res
);

  wire sign, round, normalised, zero;
  wire [8:0] exponent, sum_exponent;
  wire [22:0] product_mantissa;
  wire [23:0] op_a, op_b;
  wire [47:0] product, product_normalised;


  assign sign = a[31] ^ b[31];  // XOR of 32nd bit
  assign exception = (&a[30:23]) | (&b[30:23]);											// Execption sets to 1 when exponent of any a or b is 255
  // If exponent is 0, hidden bit is 0

  assign op_a = (|a[30:23]) ? {1'b1, a[22:0]} : {1'b0, a[22:0]};
  assign op_b = (|b[30:23]) ? {1'b1, b[22:0]} : {1'b0, b[22:0]};

  assign product = op_a * op_b;  // Product
  assign normalised = product[47] ? 1'b1 : 1'b0;
  assign product_normalised = normalised ? product : product << 1;								// Normalized value based on 48th bit

  assign round = |product_normalised[22:0];  // Last 22 bits are ORed for rounding off purpose
  assign product_mantissa = product_normalised[46:24] + (product_normalised[23] & round); 					// Mantissa

  assign zero = exception ? 1'b0 :
                (product_mantissa == 23'd0) ? 1'b1 :
                1'b0;

  assign sum_exponent = a[30:23] + b[30:23];
  assign exponent = sum_exponent - 8'd127 + normalised;
  assign overflow = ((exponent[8] & !exponent[7]) & !zero) ; 									// Overall exponent is greater than 255 then Overflow
  assign underflow = ((exponent[8] & exponent[7]) & !zero) ? 1'b1 : 1'b0; 							// Sum of exponents is less than 255 then Underflow
  assign res = exception ? 32'd0 :
               zero ? {sign,31'd0} :
               overflow ? {sign,8'hFF,23'd0} :
               underflow ? {sign,31'd0} :
               {sign,exponent[7:0],product_mantissa};

endmodule

module FP_Mult_Pipe(
	input _go, // unused
	input clk,
	input logic [31:0] A,
	input logic [31:0] B,
	output logic [31:0] res
);
  `define BIAS 8'b01111111

//////////////// PIPE-LINE REGISTERS /////////////////
reg [62:0] P1;
reg [66:0] P2;
reg [31:0] P3;
//////////////////////////////////////////////////////

initial
begin
	P1 = 0;
	P2 = 0;
	P3 = 0;
end


wire [1:0]sign;
wire [49:0]mantissa;

assign sign = A[31]+B[31];

always_ff @(posedge clk)
begin
	//solve for the sign bit part
	/////////////////////////////////////////////////////////
	P1[0] <= (sign == 1'b1) ? 1'b1 : 1'b0;
	P1[31:1] <= A[30:0];
	P1[62:32] <= B[30:0];

	///////////////////////////////////////////////////////////
	P2[0] <= P1[0];
	P2[50:1] <= P1[23:1] * P1[54:32];

	P2[58:51] <= P1[31:24];
	P2[66:59] <= P1[62:55];

///////////////////////////////////////////////////////////
	P3[0] <= P2[0];

	if(P2[50:24] == 0) begin
	   P3[23:1] = P2[23:1];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h00;
	end
	else if(P2[50] == 1) begin
	   P3[23:1] = P2[49:27];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h1a;
	end
	else if(P2[49] == 1) begin
	   P3[23:1] = P2[48:26];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h19;
	end
	else if(P2[48] == 1) begin
	   P3[23:1] = P2[47:25];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h18;
	end
	else if(P2[47] == 1) begin
	   P3[23:1] = P2[46:24];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h17;
	end
	else if(P2[46] == 1) begin
	   P3[23:1] = P2[45:23];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h16;
	end
	else if(P2[45] == 1) begin
	   P3[23:1] = P2[44:22];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h15;
	end
	else if(P2[44] == 1) begin
	   P3[23:1] = P2[43:21];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h14;
	end
	else if(P2[43] == 1) begin
	   P3[23:1] = P2[42:20];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h13;
	end
	else if(P2[42] == 1) begin
	   P3[23:1] = P2[41:19];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h12;
	end
	else if(P2[41] == 1) begin
	   P3[23:1] = P2[40:18];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h11;
	end
	else if(P2[40] == 1) begin
	   P3[23:1] = P2[39:17];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h10;
	end
	else if(P2[39] == 1) begin
	   P3[23:1] = P2[38:16];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0f;
	end
	else if(P2[38] == 1) begin
	   P3[23:1] = P2[37:15];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0e;
	end
	else if(P2[37] == 1) begin
	   P3[23:1] = P2[36:14];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0d;
	end
	else if(P2[36] == 1) begin
	   P3[23:1] = P2[35:13];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0c;
	end
	else if(P2[35] == 1) begin
	   P3[23:1] = P2[34:12];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0b;
	end
	else if(P2[34] == 1) begin
	   P3[23:1] = P2[33:11];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h0a;
	end
	else if(P2[33] == 1) begin
	   P3[23:1] = P2[32:10];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h09;
	end
	else if(P2[32] == 1) begin
	   P3[23:1] = P2[31:09];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h08;
	end
	else if(P2[31] == 1) begin
	   P3[23:1] = P2[30:08];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h07;
	end
	else if(P2[30] == 1) begin
	   P3[23:1] = P2[29:07];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h06;
	end
	else if(P2[29] == 1) begin
	   P3[23:1] = P2[28:06];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h05;
	end
	else if(P2[28] == 1) begin
	   P3[23:1] = P2[27:05];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h04;
	end
	else if(P2[27] == 1) begin
	   P3[23:1] = P2[26:04];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h03;
	end
	else if(P2[26] == 1) begin
	   P3[23:1] = P2[25:03];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h02;
	end
	else begin
	   P3[23:1] = P2[24:02];
	   P3[31:24] = P2[58:51] + P2[66:59] - `BIAS + 8'h01;
	end

///////////////////////////////////////////////////////////
	res[31] <= P3[0];
	res[30:0] <= P3[31:1];

////////////////////////////////////////////////////////////
end

endmodule

// Returns input[30:23]
module Slice30_23
(
    input logic _go,
    input logic [31:0] in,
    output logic [7:0] out
);
    assign out = in[30:23];
endmodule

// Returns in[23:1]
module Slice23_1
(
    input logic _go,
    input logic [23:0] in,
    output logic [22:0] out
);
    assign out = in[23:1];
endmodule

// Returns input[20:0]
module Slice22_0
(
    input logic _go,
    input logic [31:0] in,
    output logic [22:0] out
);
    assign out = in[22:0];
endmodule

module IsZero #(
    parameter WIDTH = 32
)
(
    input logic _go,
    input logic [WIDTH-1:0] in,
    output logic out
);
    assign out = in == '0;
endmodule

module IsOne (
    input wire logic _go,
    input wire logic in,
    output wire logic out
);
    assign out = in == '1;
endmodule

// Computes out = {1'b1,in[22:1]};
module NormInp
(
    input wire logic _go,
    input wire logic [22:0] in,
    output wire logic [22:0] out
);
    assign out = {1'b1,in[22:1]};
endmodule

module ShiftRight23
(
    input wire logic _go,
    input wire logic [22:0] in,
    input wire logic [7:0] amount,
    output wire logic [22:0] out
);
    assign out = in >> amount;
endmodule

module ShiftLeft24
(
    input wire logic _go,
    input wire logic [23:0] in,
    input wire logic [3:0] amount,
    output wire logic [23:0] out
);
    assign out = in << amount;
endmodule

module MergeRes(
    input wire logic _go,
    input wire logic sign,
    input wire logic [7:0] exp,
    input wire logic [22:0] mant,
    output wire logic [31:0] out
);
    assign out = { sign, exp, mant };
endmodule
`default_nettype none

module Latch #(
    parameter WIDTH = 32
) (
  input wire clk,
  input wire reset,
  input wire logic write_en,
  input wire logic _go_S, // unused
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] out
);
  always_ff @(posedge clk) begin
    if (reset)
      out <= 0;
    else if (write_en)
      out <= in;
    else
      out <= out;
  end
endmodule

// Implements a simple mutliplexer
module Mux #(
  parameter WIDTH = 32
) (
  input wire logic _go, // unused
  input wire logic sel,
  input wire logic [WIDTH-1:0] in0,
  input wire logic [WIDTH-1:0] in1,
  output logic [WIDTH-1:0] out
);
  assign out = sel ? in0 : in1;
endmodule
module Const #(
  parameter WIDTH = 32,
  parameter VALUE = 0
) (
  input wire logic _go,
  output wire logic [WIDTH-1:0] out
);
  assign out = VALUE;
endmodule

//// =========== Computation ============

module Add #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module Sub #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module MultComb #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left * right;
endmodule

//// =========== Binary Logical Operations ============

module And #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module Or #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module Xor #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module Not #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] in,
  output wire logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

//// =========== Comparions ============

module Gt #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left > right;
endmodule

module Eq #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left == right;
endmodule

module Lt #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left < right;
endmodule

module Gte #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left >= right;
endmodule

module Lte #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left <= right;
endmodule

//// =========== Extension ============

module ZeroExtend #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WIDTH-1:0] out
);
  parameter EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = {{EXTEND{1'b0}}, in};
endmodule

module Concat #(
  parameter LEFT = 32,
  parameter RIGHT = 32,
  parameter OUT = 64
) (
  input wire logic _go, // Unused port
  input wire logic [LEFT-1:0] left,
  input wire logic [RIGHT-1:0] right,
  output wire logic [OUT-1:0] out
);
  assign out = {left, right};
endmodule

//// =========== Select bits ============
module Select #(
  parameter WIDTH = 32,
  parameter POS = 0
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = in[POS];
endmodule

module Slice #(
  parameter IN_WIDTH = 32,
  parameter MSB = 31,
  parameter LSB = 0,
  parameter OUT_WITDH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WITDH-1:0] out
);
  assign out = in[MSB:LSB];
endmodule

/// =========== Reduction Operations ============
module ReduceAnd #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = &in;
endmodule

module ReduceOr #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = |in;
endmodule

/// ========== Shift Operations ============
module ShiftLeft #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  input wire logic [WIDTH-1:0] shift,
  output wire logic [WIDTH-1:0] out
);
  assign out = in << shift;
endmodule

module ShiftRight #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  input wire logic [WIDTH-1:0] shift,
  output wire logic [WIDTH-1:0] out
);
  assign out = in >> shift;
endmodule

`default_nettype wire
module Mult (
    input logic [31:0] left,
    input logic [31:0] right,
    input logic go_G,
    input logic clk,
    input logic reset,
    output logic [31:0] out
);
    // COMPONENT START: Mult
    logic G_fsm_clk;
    logic G_fsm_reset;
    logic G_fsm_go;
    logic G_fsm__0;
    logic G_fsm__1;
    logic G_fsm__2;
    logic [31:0] LL_in;
    logic LL_write_en;
    logic LL__go_S;
    logic LL_clk;
    logic LL_reset;
    logic [31:0] LL_out;
    logic [31:0] LR_in;
    logic LR_write_en;
    logic LR__go_S;
    logic LR_clk;
    logic LR_reset;
    logic [31:0] LR_out;
    logic [31:0] M_left;
    logic [31:0] M_right;
    logic M__go;
    logic [31:0] M_out;
    initial begin
        G_fsm_clk = 1'd0;
        G_fsm_reset = 1'd0;
        G_fsm_go = 1'd0;
        LL_in = 32'd0;
        LL_write_en = 1'd0;
        LL__go_S = 1'd0;
        LL_clk = 1'd0;
        LL_reset = 1'd0;
        LR_in = 32'd0;
        LR_write_en = 1'd0;
        LR__go_S = 1'd0;
        LR_clk = 1'd0;
        LR_reset = 1'd0;
        M_left = 32'd0;
        M_right = 32'd0;
        M__go = 1'd0;
    end
    fsm_3 G_fsm (
        ._0(G_fsm__0),
        ._1(G_fsm__1),
        ._2(G_fsm__2),
        .clk(G_fsm_clk),
        .go(G_fsm_go),
        .reset(G_fsm_reset)
    );
    Latch # (
        .WIDTH(32)
    ) LL (
        ._go_S(LL__go_S),
        .clk(LL_clk),
        .in(LL_in),
        .out(LL_out),
        .reset(LL_reset),
        .write_en(LL_write_en)
    );
    Latch # (
        .WIDTH(32)
    ) LR (
        ._go_S(LR__go_S),
        .clk(LR_clk),
        .in(LR_in),
        .out(LR_out),
        .reset(LR_reset),
        .write_en(LR_write_en)
    );
    MultComb # (
        .WIDTH(32)
    ) M (
        ._go(M__go),
        .left(M_left),
        .out(M_out),
        .right(M_right)
    );
    assign G_fsm_clk = clk;
    assign G_fsm_go = go_G;
    assign G_fsm_reset = reset;
    assign LL__go_S = G_fsm__1;
    assign LL_clk = clk;
    assign LL_in =
     G_fsm__0 ? left : 32'd0;
    assign LL_reset = reset;
    assign LL_write_en = G_fsm__0;
    assign LR__go_S = G_fsm__1;
    assign LR_clk = clk;
    assign LR_in =
     G_fsm__0 ? right : 32'd0;
    assign LR_reset = reset;
    assign LR_write_en = G_fsm__0;
    assign M__go = G_fsm__2;
    assign M_left =
     G_fsm__2 ? LL_out : 32'd0;
    assign M_right =
     G_fsm__2 ? LR_out : 32'd0;
    assign out = M_out;
    // COMPONENT END: Mult
endmodule

module ComputeOp (
    input logic [31:0] in,
    input logic go,
    input logic clk,
    input logic reset,
    output logic [23:0] out
);
    // COMPONENT START: ComputeOp
    logic G_fsm_clk;
    logic G_fsm_reset;
    logic G_fsm_go;
    logic G_fsm__0;
    logic [31:0] EXP_in;
    logic EXP__go;
    logic [7:0] EXP_out;
    logic [31:0] MANT_in;
    logic MANT__go;
    logic [22:0] MANT_out;
    logic [7:0] EXP_RED_in;
    logic EXP_RED__go;
    logic EXP_RED_out;
    logic ZERO_1__go;
    logic ZERO_1_out;
    logic ONE_1__go;
    logic ONE_1_out;
    logic EXT_sel;
    logic EXT_in0;
    logic EXT_in1;
    logic EXT__go;
    logic EXT_out;
    logic OP_left;
    logic [22:0] OP_right;
    logic OP__go;
    logic [23:0] OP_out;
    initial begin
        G_fsm_clk = 1'd0;
        G_fsm_reset = 1'd0;
        G_fsm_go = 1'd0;
        EXP_in = 32'd0;
        EXP__go = 1'd0;
        MANT_in = 32'd0;
        MANT__go = 1'd0;
        EXP_RED_in = 8'd0;
        EXP_RED__go = 1'd0;
        ZERO_1__go = 1'd0;
        ONE_1__go = 1'd0;
        EXT_sel = 1'd0;
        EXT_in0 = 1'd0;
        EXT_in1 = 1'd0;
        EXT__go = 1'd0;
        OP_left = 1'd0;
        OP_right = 23'd0;
        OP__go = 1'd0;
    end
    fsm_1 G_fsm (
        ._0(G_fsm__0),
        .clk(G_fsm_clk),
        .go(G_fsm_go),
        .reset(G_fsm_reset)
    );
    Slice30_23 EXP (
        ._go(EXP__go),
        .in(EXP_in),
        .out(EXP_out)
    );
    Slice22_0 MANT (
        ._go(MANT__go),
        .in(MANT_in),
        .out(MANT_out)
    );
    ReduceOr # (
        .WIDTH(8)
    ) EXP_RED (
        ._go(EXP_RED__go),
        .in(EXP_RED_in),
        .out(EXP_RED_out)
    );
    Const # (
        .VALUE(0),
        .WIDTH(1)
    ) ZERO_1 (
        ._go(ZERO_1__go),
        .out(ZERO_1_out)
    );
    Const # (
        .VALUE(1),
        .WIDTH(1)
    ) ONE_1 (
        ._go(ONE_1__go),
        .out(ONE_1_out)
    );
    Mux # (
        .WIDTH(1)
    ) EXT (
        ._go(EXT__go),
        .in0(EXT_in0),
        .in1(EXT_in1),
        .out(EXT_out),
        .sel(EXT_sel)
    );
    Concat # (
        .LEFT(1),
        .OUT(24),
        .RIGHT(23)
    ) OP (
        ._go(OP__go),
        .left(OP_left),
        .out(OP_out),
        .right(OP_right)
    );
    assign EXP__go = G_fsm__0;
    assign EXP_in =
     G_fsm__0 ? in : 32'd0;
    assign EXP_RED__go = G_fsm__0;
    assign EXP_RED_in =
     G_fsm__0 ? EXP_out : 8'd0;
    assign EXT__go = G_fsm__0;
    assign EXT_in0 =
     G_fsm__0 ? ONE_1_out : 1'd0;
    assign EXT_in1 =
     G_fsm__0 ? ZERO_1_out : 1'd0;
    assign EXT_sel =
     G_fsm__0 ? EXP_RED_out : 1'd0;
    assign G_fsm_clk = clk;
    assign G_fsm_go = go;
    assign G_fsm_reset = reset;
    assign MANT__go = G_fsm__0;
    assign MANT_in =
     G_fsm__0 ? in : 32'd0;
    assign ONE_1__go = G_fsm__0;
    assign OP__go = G_fsm__0;
    assign OP_left =
     G_fsm__0 ? EXT_out : 1'd0;
    assign OP_right =
     G_fsm__0 ? MANT_out : 23'd0;
    assign ZERO_1__go = G_fsm__0;
    assign out = OP_out;
    // COMPONENT END: ComputeOp
endmodule

module GetResult (
    input logic sign,
    input logic [7:0] exp,
    input logic [22:0] mant,
    input logic go,
    input logic reset,
    input logic clk,
    output logic [31:0] out
);
    // COMPONENT START: GetResult
    logic G_fsm_clk;
    logic G_fsm_reset;
    logic G_fsm_go;
    logic G_fsm__0;
    logic SIGN_EXP_left;
    logic [7:0] SIGN_EXP_right;
    logic SIGN_EXP__go;
    logic [8:0] SIGN_EXP_out;
    logic [8:0] RES_left;
    logic [22:0] RES_right;
    logic RES__go;
    logic [31:0] RES_out;
    initial begin
        G_fsm_clk = 1'd0;
        G_fsm_reset = 1'd0;
        G_fsm_go = 1'd0;
        SIGN_EXP_left = 1'd0;
        SIGN_EXP_right = 8'd0;
        SIGN_EXP__go = 1'd0;
        RES_left = 9'd0;
        RES_right = 23'd0;
        RES__go = 1'd0;
    end
    fsm_1 G_fsm (
        ._0(G_fsm__0),
        .clk(G_fsm_clk),
        .go(G_fsm_go),
        .reset(G_fsm_reset)
    );
    Concat # (
        .LEFT(1),
        .OUT(9),
        .RIGHT(8)
    ) SIGN_EXP (
        ._go(SIGN_EXP__go),
        .left(SIGN_EXP_left),
        .out(SIGN_EXP_out),
        .right(SIGN_EXP_right)
    );
    Concat # (
        .LEFT(9),
        .OUT(32),
        .RIGHT(23)
    ) RES (
        ._go(RES__go),
        .left(RES_left),
        .out(RES_out),
        .right(RES_right)
    );
    assign G_fsm_clk = clk;
    assign G_fsm_go = go;
    assign G_fsm_reset = reset;
    assign RES__go = G_fsm__0;
    assign RES_left =
     G_fsm__0 ? SIGN_EXP_out : 9'd0;
    assign RES_right =
     G_fsm__0 ? mant : 23'd0;
    assign SIGN_EXP__go = G_fsm__0;
    assign SIGN_EXP_left =
     G_fsm__0 ? sign : 1'd0;
    assign SIGN_EXP_right =
     G_fsm__0 ? exp : 8'd0;
    assign out = RES_out;
    // COMPONENT END: GetResult
endmodule

module FP_Mult (
    input logic [31:0] a,
    input logic [31:0] b,
    input logic go,
    input logic reset,
    input logic clk,
    output logic exception,
    output logic overflow,
    output logic underflow,
    output logic [31:0] out
);
    // COMPONENT START: FP_Mult
    logic G_fsm_clk;
    logic G_fsm_reset;
    logic G_fsm_go;
    logic G_fsm__0;
    logic [31:0] A31_in;
    logic A31__go;
    logic A31_out;
    logic [31:0] B31_in;
    logic B31__go;
    logic B31_out;
    logic SIGN_left;
    logic SIGN_right;
    logic SIGN__go;
    logic SIGN_out;
    logic [31:0] EXP_A_in;
    logic EXP_A__go;
    logic [7:0] EXP_A_out;
    logic [31:0] EXP_B_in;
    logic EXP_B__go;
    logic [7:0] EXP_B_out;
    logic [7:0] EXP_A_AND_in;
    logic EXP_A_AND__go;
    logic EXP_A_AND_out;
    logic [7:0] EXP_B_AND_in;
    logic EXP_B_AND__go;
    logic EXP_B_AND_out;
    logic EXCEPTION_left;
    logic EXCEPTION_right;
    logic EXCEPTION__go;
    logic EXCEPTION_out;
    logic [31:0] OP_A_in;
    logic OP_A_go;
    logic OP_A_clk;
    logic OP_A_reset;
    logic [23:0] OP_A_out;
    logic [31:0] OP_B_in;
    logic OP_B_go;
    logic OP_B_clk;
    logic OP_B_reset;
    logic [23:0] OP_B_out;
    logic [23:0] OP_A_EXT_in;
    logic OP_A_EXT__go;
    logic [47:0] OP_A_EXT_out;
    logic [23:0] OP_B_EXT_in;
    logic OP_B_EXT__go;
    logic [47:0] OP_B_EXT_out;
    logic [47:0] PRODUCT_left;
    logic [47:0] PRODUCT_right;
    logic PRODUCT__go;
    logic [47:0] PRODUCT_out;
    logic [47:0] NORMALIZED_in;
    logic NORMALIZED__go;
    logic NORMALIZED_out;
    logic ONE_48__go;
    logic [47:0] ONE_48_out;
    logic [47:0] PROD_SHL_in;
    logic [47:0] PROD_SHL_shift;
    logic PROD_SHL__go;
    logic [47:0] PROD_SHL_out;
    logic PRODUCT_NORMALIZED_sel;
    logic [47:0] PRODUCT_NORMALIZED_in0;
    logic [47:0] PRODUCT_NORMALIZED_in1;
    logic PRODUCT_NORMALIZED__go;
    logic [47:0] PRODUCT_NORMALIZED_out;
    logic [47:0] PROD22_0_in;
    logic PROD22_0__go;
    logic [22:0] PROD22_0_out;
    logic [22:0] ROUND_in;
    logic ROUND__go;
    logic ROUND_out;
    logic [47:0] PROD_NORM_23_in;
    logic PROD_NORM_23__go;
    logic PROD_NORM_23_out;
    logic ROUND_SUM_left;
    logic ROUND_SUM_right;
    logic ROUND_SUM__go;
    logic ROUND_SUM_out;
    logic ROUND_SUM_EXT_in;
    logic ROUND_SUM_EXT__go;
    logic [22:0] ROUND_SUM_EXT_out;
    logic [47:0] PROD46_24_in;
    logic PROD46_24__go;
    logic [22:0] PROD46_24_out;
    logic [22:0] PRODUCT_MANTISSA_left;
    logic [22:0] PRODUCT_MANTISSA_right;
    logic PRODUCT_MANTISSA__go;
    logic [22:0] PRODUCT_MANTISSA_out;
    logic ZERO_23__go;
    logic [22:0] ZERO_23_out;
    logic ONE_1__go;
    logic ONE_1_out;
    logic ZERO_1__go;
    logic ZERO_1_out;
    logic [22:0] PROD_MANT_IS_ZERO_left;
    logic [22:0] PROD_MANT_IS_ZERO_right;
    logic PROD_MANT_IS_ZERO__go;
    logic PROD_MANT_IS_ZERO_out;
    logic SEL0_sel;
    logic SEL0_in0;
    logic SEL0_in1;
    logic SEL0__go;
    logic SEL0_out;
    logic ZERO_sel;
    logic ZERO_in0;
    logic ZERO_in1;
    logic ZERO__go;
    logic ZERO_out;
    logic [7:0] SUM_EXPONENT_left;
    logic [7:0] SUM_EXPONENT_right;
    logic SUM_EXPONENT__go;
    logic [7:0] SUM_EXPONENT_out;
    logic NORMALIZED_EXT_in;
    logic NORMALIZED_EXT__go;
    logic [8:0] NORMALIZED_EXT_out;
    logic SUB_V__go;
    logic [7:0] SUB_V_out;
    logic [7:0] EXP_NORM_left;
    logic [7:0] EXP_NORM_right;
    logic EXP_NORM__go;
    logic [7:0] EXP_NORM_out;
    logic [7:0] EXP_NORM_EXT_in;
    logic EXP_NORM_EXT__go;
    logic [8:0] EXP_NORM_EXT_out;
    logic [8:0] EXPONENT_left;
    logic [8:0] EXPONENT_right;
    logic EXPONENT__go;
    logic [8:0] EXPONENT_out;
    logic NOT_ZERO_in;
    logic NOT_ZERO__go;
    logic NOT_ZERO_out;
    logic [8:0] EXP_8_in;
    logic EXP_8__go;
    logic EXP_8_out;
    logic [8:0] EXP_7_in;
    logic EXP_7__go;
    logic EXP_7_out;
    logic NOT_EXP_7_in;
    logic NOT_EXP_7__go;
    logic NOT_EXP_7_out;
    logic OVERFLOW_1_left;
    logic OVERFLOW_1_right;
    logic OVERFLOW_1__go;
    logic OVERFLOW_1_out;
    logic OVERFLOW_left;
    logic OVERFLOW_right;
    logic OVERFLOW__go;
    logic OVERFLOW_out;
    logic UNDERFLOW_1_left;
    logic UNDERFLOW_1_right;
    logic UNDERFLOW_1__go;
    logic UNDERFLOW_1_out;
    logic UNDERFLOW_left;
    logic UNDERFLOW_right;
    logic UNDERFLOW__go;
    logic UNDERFLOW_out;
    logic OVER_MAX__go;
    logic [7:0] OVER_MAX_out;
    logic OVER_RES_sign;
    logic [7:0] OVER_RES_exp;
    logic [22:0] OVER_RES_mant;
    logic OVER_RES_go;
    logic OVER_RES_reset;
    logic OVER_RES_clk;
    logic [31:0] OVER_RES_out;
    logic ZERO_31__go;
    logic [30:0] ZERO_31_out;
    logic UNDER_RES_left;
    logic [30:0] UNDER_RES_right;
    logic UNDER_RES__go;
    logic [31:0] UNDER_RES_out;
    logic [8:0] EXP_ROUND_in;
    logic EXP_ROUND__go;
    logic [7:0] EXP_ROUND_out;
    logic SAFE_RES_sign;
    logic [7:0] SAFE_RES_exp;
    logic [22:0] SAFE_RES_mant;
    logic SAFE_RES_go;
    logic SAFE_RES_reset;
    logic SAFE_RES_clk;
    logic [31:0] SAFE_RES_out;
    logic ZERO_32__go;
    logic [31:0] ZERO_32_out;
    logic RES_1_sel;
    logic [31:0] RES_1_in0;
    logic [31:0] RES_1_in1;
    logic RES_1__go;
    logic [31:0] RES_1_out;
    logic RES_2_sel;
    logic [31:0] RES_2_in0;
    logic [31:0] RES_2_in1;
    logic RES_2__go;
    logic [31:0] RES_2_out;
    logic RES_3_sel;
    logic [31:0] RES_3_in0;
    logic [31:0] RES_3_in1;
    logic RES_3__go;
    logic [31:0] RES_3_out;
    logic RES_sel;
    logic [31:0] RES_in0;
    logic [31:0] RES_in1;
    logic RES__go;
    logic [31:0] RES_out;
    initial begin
        G_fsm_clk = 1'd0;
        G_fsm_reset = 1'd0;
        G_fsm_go = 1'd0;
        A31_in = 32'd0;
        A31__go = 1'd0;
        B31_in = 32'd0;
        B31__go = 1'd0;
        SIGN_left = 1'd0;
        SIGN_right = 1'd0;
        SIGN__go = 1'd0;
        EXP_A_in = 32'd0;
        EXP_A__go = 1'd0;
        EXP_B_in = 32'd0;
        EXP_B__go = 1'd0;
        EXP_A_AND_in = 8'd0;
        EXP_A_AND__go = 1'd0;
        EXP_B_AND_in = 8'd0;
        EXP_B_AND__go = 1'd0;
        EXCEPTION_left = 1'd0;
        EXCEPTION_right = 1'd0;
        EXCEPTION__go = 1'd0;
        OP_A_in = 32'd0;
        OP_A_go = 1'd0;
        OP_A_clk = 1'd0;
        OP_A_reset = 1'd0;
        OP_B_in = 32'd0;
        OP_B_go = 1'd0;
        OP_B_clk = 1'd0;
        OP_B_reset = 1'd0;
        OP_A_EXT_in = 24'd0;
        OP_A_EXT__go = 1'd0;
        OP_B_EXT_in = 24'd0;
        OP_B_EXT__go = 1'd0;
        PRODUCT_left = 48'd0;
        PRODUCT_right = 48'd0;
        PRODUCT__go = 1'd0;
        NORMALIZED_in = 48'd0;
        NORMALIZED__go = 1'd0;
        ONE_48__go = 1'd0;
        PROD_SHL_in = 48'd0;
        PROD_SHL_shift = 48'd0;
        PROD_SHL__go = 1'd0;
        PRODUCT_NORMALIZED_sel = 1'd0;
        PRODUCT_NORMALIZED_in0 = 48'd0;
        PRODUCT_NORMALIZED_in1 = 48'd0;
        PRODUCT_NORMALIZED__go = 1'd0;
        PROD22_0_in = 48'd0;
        PROD22_0__go = 1'd0;
        ROUND_in = 23'd0;
        ROUND__go = 1'd0;
        PROD_NORM_23_in = 48'd0;
        PROD_NORM_23__go = 1'd0;
        ROUND_SUM_left = 1'd0;
        ROUND_SUM_right = 1'd0;
        ROUND_SUM__go = 1'd0;
        ROUND_SUM_EXT_in = 1'd0;
        ROUND_SUM_EXT__go = 1'd0;
        PROD46_24_in = 48'd0;
        PROD46_24__go = 1'd0;
        PRODUCT_MANTISSA_left = 23'd0;
        PRODUCT_MANTISSA_right = 23'd0;
        PRODUCT_MANTISSA__go = 1'd0;
        ZERO_23__go = 1'd0;
        ONE_1__go = 1'd0;
        ZERO_1__go = 1'd0;
        PROD_MANT_IS_ZERO_left = 23'd0;
        PROD_MANT_IS_ZERO_right = 23'd0;
        PROD_MANT_IS_ZERO__go = 1'd0;
        SEL0_sel = 1'd0;
        SEL0_in0 = 1'd0;
        SEL0_in1 = 1'd0;
        SEL0__go = 1'd0;
        ZERO_sel = 1'd0;
        ZERO_in0 = 1'd0;
        ZERO_in1 = 1'd0;
        ZERO__go = 1'd0;
        SUM_EXPONENT_left = 8'd0;
        SUM_EXPONENT_right = 8'd0;
        SUM_EXPONENT__go = 1'd0;
        NORMALIZED_EXT_in = 1'd0;
        NORMALIZED_EXT__go = 1'd0;
        SUB_V__go = 1'd0;
        EXP_NORM_left = 8'd0;
        EXP_NORM_right = 8'd0;
        EXP_NORM__go = 1'd0;
        EXP_NORM_EXT_in = 8'd0;
        EXP_NORM_EXT__go = 1'd0;
        EXPONENT_left = 9'd0;
        EXPONENT_right = 9'd0;
        EXPONENT__go = 1'd0;
        NOT_ZERO_in = 1'd0;
        NOT_ZERO__go = 1'd0;
        EXP_8_in = 9'd0;
        EXP_8__go = 1'd0;
        EXP_7_in = 9'd0;
        EXP_7__go = 1'd0;
        NOT_EXP_7_in = 1'd0;
        NOT_EXP_7__go = 1'd0;
        OVERFLOW_1_left = 1'd0;
        OVERFLOW_1_right = 1'd0;
        OVERFLOW_1__go = 1'd0;
        OVERFLOW_left = 1'd0;
        OVERFLOW_right = 1'd0;
        OVERFLOW__go = 1'd0;
        UNDERFLOW_1_left = 1'd0;
        UNDERFLOW_1_right = 1'd0;
        UNDERFLOW_1__go = 1'd0;
        UNDERFLOW_left = 1'd0;
        UNDERFLOW_right = 1'd0;
        UNDERFLOW__go = 1'd0;
        OVER_MAX__go = 1'd0;
        OVER_RES_sign = 1'd0;
        OVER_RES_exp = 8'd0;
        OVER_RES_mant = 23'd0;
        OVER_RES_go = 1'd0;
        OVER_RES_reset = 1'd0;
        OVER_RES_clk = 1'd0;
        ZERO_31__go = 1'd0;
        UNDER_RES_left = 1'd0;
        UNDER_RES_right = 31'd0;
        UNDER_RES__go = 1'd0;
        EXP_ROUND_in = 9'd0;
        EXP_ROUND__go = 1'd0;
        SAFE_RES_sign = 1'd0;
        SAFE_RES_exp = 8'd0;
        SAFE_RES_mant = 23'd0;
        SAFE_RES_go = 1'd0;
        SAFE_RES_reset = 1'd0;
        SAFE_RES_clk = 1'd0;
        ZERO_32__go = 1'd0;
        RES_1_sel = 1'd0;
        RES_1_in0 = 32'd0;
        RES_1_in1 = 32'd0;
        RES_1__go = 1'd0;
        RES_2_sel = 1'd0;
        RES_2_in0 = 32'd0;
        RES_2_in1 = 32'd0;
        RES_2__go = 1'd0;
        RES_3_sel = 1'd0;
        RES_3_in0 = 32'd0;
        RES_3_in1 = 32'd0;
        RES_3__go = 1'd0;
        RES_sel = 1'd0;
        RES_in0 = 32'd0;
        RES_in1 = 32'd0;
        RES__go = 1'd0;
    end
    fsm_1 G_fsm (
        ._0(G_fsm__0),
        .clk(G_fsm_clk),
        .go(G_fsm_go),
        .reset(G_fsm_reset)
    );
    Select # (
        .POS(31),
        .WIDTH(32)
    ) A31 (
        ._go(A31__go),
        .in(A31_in),
        .out(A31_out)
    );
    Select # (
        .POS(31),
        .WIDTH(32)
    ) B31 (
        ._go(B31__go),
        .in(B31_in),
        .out(B31_out)
    );
    Xor # (
        .WIDTH(1)
    ) SIGN (
        ._go(SIGN__go),
        .left(SIGN_left),
        .out(SIGN_out),
        .right(SIGN_right)
    );
    Slice30_23 EXP_A (
        ._go(EXP_A__go),
        .in(EXP_A_in),
        .out(EXP_A_out)
    );
    Slice30_23 EXP_B (
        ._go(EXP_B__go),
        .in(EXP_B_in),
        .out(EXP_B_out)
    );
    ReduceAnd # (
        .WIDTH(8)
    ) EXP_A_AND (
        ._go(EXP_A_AND__go),
        .in(EXP_A_AND_in),
        .out(EXP_A_AND_out)
    );
    ReduceAnd # (
        .WIDTH(8)
    ) EXP_B_AND (
        ._go(EXP_B_AND__go),
        .in(EXP_B_AND_in),
        .out(EXP_B_AND_out)
    );
    Or # (
        .WIDTH(1)
    ) EXCEPTION (
        ._go(EXCEPTION__go),
        .left(EXCEPTION_left),
        .out(EXCEPTION_out),
        .right(EXCEPTION_right)
    );
    ComputeOp OP_A (
        .clk(OP_A_clk),
        .go(OP_A_go),
        .in(OP_A_in),
        .out(OP_A_out),
        .reset(OP_A_reset)
    );
    ComputeOp OP_B (
        .clk(OP_B_clk),
        .go(OP_B_go),
        .in(OP_B_in),
        .out(OP_B_out),
        .reset(OP_B_reset)
    );
    ZeroExtend # (
        .IN_WIDTH(24),
        .OUT_WIDTH(48)
    ) OP_A_EXT (
        ._go(OP_A_EXT__go),
        .in(OP_A_EXT_in),
        .out(OP_A_EXT_out)
    );
    ZeroExtend # (
        .IN_WIDTH(24),
        .OUT_WIDTH(48)
    ) OP_B_EXT (
        ._go(OP_B_EXT__go),
        .in(OP_B_EXT_in),
        .out(OP_B_EXT_out)
    );
    MultComb # (
        .WIDTH(48)
    ) PRODUCT (
        ._go(PRODUCT__go),
        .left(PRODUCT_left),
        .out(PRODUCT_out),
        .right(PRODUCT_right)
    );
    Select # (
        .POS(47),
        .WIDTH(48)
    ) NORMALIZED (
        ._go(NORMALIZED__go),
        .in(NORMALIZED_in),
        .out(NORMALIZED_out)
    );
    Const # (
        .VALUE(1),
        .WIDTH(48)
    ) ONE_48 (
        ._go(ONE_48__go),
        .out(ONE_48_out)
    );
    ShiftLeft # (
        .WIDTH(48)
    ) PROD_SHL (
        ._go(PROD_SHL__go),
        .in(PROD_SHL_in),
        .out(PROD_SHL_out),
        .shift(PROD_SHL_shift)
    );
    Mux # (
        .WIDTH(48)
    ) PRODUCT_NORMALIZED (
        ._go(PRODUCT_NORMALIZED__go),
        .in0(PRODUCT_NORMALIZED_in0),
        .in1(PRODUCT_NORMALIZED_in1),
        .out(PRODUCT_NORMALIZED_out),
        .sel(PRODUCT_NORMALIZED_sel)
    );
    Slice # (
        .IN_WIDTH(48),
        .LSB(0),
        .MSB(22),
        .OUT_WIDTH(23)
    ) PROD22_0 (
        ._go(PROD22_0__go),
        .in(PROD22_0_in),
        .out(PROD22_0_out)
    );
    ReduceOr # (
        .WIDTH(23)
    ) ROUND (
        ._go(ROUND__go),
        .in(ROUND_in),
        .out(ROUND_out)
    );
    Select # (
        .POS(23),
        .WIDTH(48)
    ) PROD_NORM_23 (
        ._go(PROD_NORM_23__go),
        .in(PROD_NORM_23_in),
        .out(PROD_NORM_23_out)
    );
    And # (
        .WIDTH(1)
    ) ROUND_SUM (
        ._go(ROUND_SUM__go),
        .left(ROUND_SUM_left),
        .out(ROUND_SUM_out),
        .right(ROUND_SUM_right)
    );
    ZeroExtend # (
        .IN_WIDTH(1),
        .OUT_WIDTH(23)
    ) ROUND_SUM_EXT (
        ._go(ROUND_SUM_EXT__go),
        .in(ROUND_SUM_EXT_in),
        .out(ROUND_SUM_EXT_out)
    );
    Slice # (
        .IN_WIDTH(48),
        .LSB(24),
        .MSB(46),
        .OUT_WIDTH(23)
    ) PROD46_24 (
        ._go(PROD46_24__go),
        .in(PROD46_24_in),
        .out(PROD46_24_out)
    );
    Add # (
        .WIDTH(23)
    ) PRODUCT_MANTISSA (
        ._go(PRODUCT_MANTISSA__go),
        .left(PRODUCT_MANTISSA_left),
        .out(PRODUCT_MANTISSA_out),
        .right(PRODUCT_MANTISSA_right)
    );
    Const # (
        .VALUE(0),
        .WIDTH(23)
    ) ZERO_23 (
        ._go(ZERO_23__go),
        .out(ZERO_23_out)
    );
    Const # (
        .VALUE(1),
        .WIDTH(1)
    ) ONE_1 (
        ._go(ONE_1__go),
        .out(ONE_1_out)
    );
    Const # (
        .VALUE(0),
        .WIDTH(1)
    ) ZERO_1 (
        ._go(ZERO_1__go),
        .out(ZERO_1_out)
    );
    Eq # (
        .WIDTH(23)
    ) PROD_MANT_IS_ZERO (
        ._go(PROD_MANT_IS_ZERO__go),
        .left(PROD_MANT_IS_ZERO_left),
        .out(PROD_MANT_IS_ZERO_out),
        .right(PROD_MANT_IS_ZERO_right)
    );
    Mux # (
        .WIDTH(1)
    ) SEL0 (
        ._go(SEL0__go),
        .in0(SEL0_in0),
        .in1(SEL0_in1),
        .out(SEL0_out),
        .sel(SEL0_sel)
    );
    Mux # (
        .WIDTH(1)
    ) ZERO (
        ._go(ZERO__go),
        .in0(ZERO_in0),
        .in1(ZERO_in1),
        .out(ZERO_out),
        .sel(ZERO_sel)
    );
    Add # (
        .WIDTH(8)
    ) SUM_EXPONENT (
        ._go(SUM_EXPONENT__go),
        .left(SUM_EXPONENT_left),
        .out(SUM_EXPONENT_out),
        .right(SUM_EXPONENT_right)
    );
    ZeroExtend # (
        .IN_WIDTH(1),
        .OUT_WIDTH(9)
    ) NORMALIZED_EXT (
        ._go(NORMALIZED_EXT__go),
        .in(NORMALIZED_EXT_in),
        .out(NORMALIZED_EXT_out)
    );
    Const # (
        .VALUE(127),
        .WIDTH(8)
    ) SUB_V (
        ._go(SUB_V__go),
        .out(SUB_V_out)
    );
    Sub # (
        .WIDTH(8)
    ) EXP_NORM (
        ._go(EXP_NORM__go),
        .left(EXP_NORM_left),
        .out(EXP_NORM_out),
        .right(EXP_NORM_right)
    );
    ZeroExtend # (
        .IN_WIDTH(8),
        .OUT_WIDTH(9)
    ) EXP_NORM_EXT (
        ._go(EXP_NORM_EXT__go),
        .in(EXP_NORM_EXT_in),
        .out(EXP_NORM_EXT_out)
    );
    Add # (
        .WIDTH(9)
    ) EXPONENT (
        ._go(EXPONENT__go),
        .left(EXPONENT_left),
        .out(EXPONENT_out),
        .right(EXPONENT_right)
    );
    Not # (
        .WIDTH(1)
    ) NOT_ZERO (
        ._go(NOT_ZERO__go),
        .in(NOT_ZERO_in),
        .out(NOT_ZERO_out)
    );
    Select # (
        .POS(8),
        .WIDTH(9)
    ) EXP_8 (
        ._go(EXP_8__go),
        .in(EXP_8_in),
        .out(EXP_8_out)
    );
    Select # (
        .POS(7),
        .WIDTH(9)
    ) EXP_7 (
        ._go(EXP_7__go),
        .in(EXP_7_in),
        .out(EXP_7_out)
    );
    Not # (
        .WIDTH(1)
    ) NOT_EXP_7 (
        ._go(NOT_EXP_7__go),
        .in(NOT_EXP_7_in),
        .out(NOT_EXP_7_out)
    );
    And # (
        .WIDTH(1)
    ) OVERFLOW_1 (
        ._go(OVERFLOW_1__go),
        .left(OVERFLOW_1_left),
        .out(OVERFLOW_1_out),
        .right(OVERFLOW_1_right)
    );
    And # (
        .WIDTH(1)
    ) OVERFLOW (
        ._go(OVERFLOW__go),
        .left(OVERFLOW_left),
        .out(OVERFLOW_out),
        .right(OVERFLOW_right)
    );
    And # (
        .WIDTH(1)
    ) UNDERFLOW_1 (
        ._go(UNDERFLOW_1__go),
        .left(UNDERFLOW_1_left),
        .out(UNDERFLOW_1_out),
        .right(UNDERFLOW_1_right)
    );
    And # (
        .WIDTH(1)
    ) UNDERFLOW (
        ._go(UNDERFLOW__go),
        .left(UNDERFLOW_left),
        .out(UNDERFLOW_out),
        .right(UNDERFLOW_right)
    );
    Const # (
        .VALUE(255),
        .WIDTH(8)
    ) OVER_MAX (
        ._go(OVER_MAX__go),
        .out(OVER_MAX_out)
    );
    GetResult OVER_RES (
        .clk(OVER_RES_clk),
        .exp(OVER_RES_exp),
        .go(OVER_RES_go),
        .mant(OVER_RES_mant),
        .out(OVER_RES_out),
        .reset(OVER_RES_reset),
        .sign(OVER_RES_sign)
    );
    Const # (
        .VALUE(0),
        .WIDTH(31)
    ) ZERO_31 (
        ._go(ZERO_31__go),
        .out(ZERO_31_out)
    );
    Concat # (
        .LEFT(1),
        .OUT(32),
        .RIGHT(31)
    ) UNDER_RES (
        ._go(UNDER_RES__go),
        .left(UNDER_RES_left),
        .out(UNDER_RES_out),
        .right(UNDER_RES_right)
    );
    Slice # (
        .IN_WIDTH(9),
        .LSB(0),
        .MSB(7),
        .OUT_WIDTH(8)
    ) EXP_ROUND (
        ._go(EXP_ROUND__go),
        .in(EXP_ROUND_in),
        .out(EXP_ROUND_out)
    );
    GetResult SAFE_RES (
        .clk(SAFE_RES_clk),
        .exp(SAFE_RES_exp),
        .go(SAFE_RES_go),
        .mant(SAFE_RES_mant),
        .out(SAFE_RES_out),
        .reset(SAFE_RES_reset),
        .sign(SAFE_RES_sign)
    );
    Const # (
        .VALUE(0),
        .WIDTH(32)
    ) ZERO_32 (
        ._go(ZERO_32__go),
        .out(ZERO_32_out)
    );
    Mux # (
        .WIDTH(32)
    ) RES_1 (
        ._go(RES_1__go),
        .in0(RES_1_in0),
        .in1(RES_1_in1),
        .out(RES_1_out),
        .sel(RES_1_sel)
    );
    Mux # (
        .WIDTH(32)
    ) RES_2 (
        ._go(RES_2__go),
        .in0(RES_2_in0),
        .in1(RES_2_in1),
        .out(RES_2_out),
        .sel(RES_2_sel)
    );
    Mux # (
        .WIDTH(32)
    ) RES_3 (
        ._go(RES_3__go),
        .in0(RES_3_in0),
        .in1(RES_3_in1),
        .out(RES_3_out),
        .sel(RES_3_sel)
    );
    Mux # (
        .WIDTH(32)
    ) RES (
        ._go(RES__go),
        .in0(RES_in0),
        .in1(RES_in1),
        .out(RES_out),
        .sel(RES_sel)
    );
    assign A31__go = G_fsm__0;
    assign A31_in =
     G_fsm__0 ? a : 32'd0;
    assign B31__go = G_fsm__0;
    assign B31_in =
     G_fsm__0 ? b : 32'd0;
    assign EXCEPTION__go = G_fsm__0;
    assign EXCEPTION_left =
     G_fsm__0 ? EXP_A_AND_out : 1'd0;
    assign EXCEPTION_right =
     G_fsm__0 ? EXP_B_AND_out : 1'd0;
    assign EXPONENT__go = G_fsm__0;
    assign EXPONENT_left =
     G_fsm__0 ? EXP_NORM_EXT_out : 9'd0;
    assign EXPONENT_right =
     G_fsm__0 ? NORMALIZED_EXT_out : 9'd0;
    assign EXP_7__go = G_fsm__0;
    assign EXP_7_in =
     G_fsm__0 ? EXPONENT_out : 9'd0;
    assign EXP_8__go = G_fsm__0;
    assign EXP_8_in =
     G_fsm__0 ? EXPONENT_out : 9'd0;
    assign EXP_A__go = G_fsm__0;
    assign EXP_A_in =
     G_fsm__0 ? a : 32'd0;
    assign EXP_A_AND__go = G_fsm__0;
    assign EXP_A_AND_in =
     G_fsm__0 ? EXP_A_out : 8'd0;
    assign EXP_B__go = G_fsm__0;
    assign EXP_B_in =
     G_fsm__0 ? b : 32'd0;
    assign EXP_B_AND__go = G_fsm__0;
    assign EXP_B_AND_in =
     G_fsm__0 ? EXP_B_out : 8'd0;
    assign EXP_NORM__go = G_fsm__0;
    assign EXP_NORM_left =
     G_fsm__0 ? SUM_EXPONENT_out : 8'd0;
    assign EXP_NORM_right =
     G_fsm__0 ? SUB_V_out : 8'd0;
    assign EXP_NORM_EXT__go = G_fsm__0;
    assign EXP_NORM_EXT_in =
     G_fsm__0 ? EXP_NORM_out : 8'd0;
    assign EXP_ROUND__go = G_fsm__0;
    assign EXP_ROUND_in =
     G_fsm__0 ? EXPONENT_out : 9'd0;
    assign G_fsm_clk = clk;
    assign G_fsm_go = go;
    assign G_fsm_reset = reset;
    assign NORMALIZED__go = G_fsm__0;
    assign NORMALIZED_in =
     G_fsm__0 ? PRODUCT_out : 48'd0;
    assign NORMALIZED_EXT__go = G_fsm__0;
    assign NORMALIZED_EXT_in =
     G_fsm__0 ? NORMALIZED_out : 1'd0;
    assign NOT_EXP_7__go = G_fsm__0;
    assign NOT_EXP_7_in =
     G_fsm__0 ? EXP_7_out : 1'd0;
    assign NOT_ZERO__go = G_fsm__0;
    assign NOT_ZERO_in =
     G_fsm__0 ? ZERO_out : 1'd0;
    assign ONE_1__go = G_fsm__0;
    assign ONE_48__go = G_fsm__0;
    assign OP_A_clk = clk;
    assign OP_A_go = G_fsm__0;
    assign OP_A_in =
     G_fsm__0 ? a : 32'd0;
    assign OP_A_reset = reset;
    assign OP_A_EXT__go = G_fsm__0;
    assign OP_A_EXT_in =
     G_fsm__0 ? OP_A_out : 24'd0;
    assign OP_B_clk = clk;
    assign OP_B_go = G_fsm__0;
    assign OP_B_in =
     G_fsm__0 ? b : 32'd0;
    assign OP_B_reset = reset;
    assign OP_B_EXT__go = G_fsm__0;
    assign OP_B_EXT_in =
     G_fsm__0 ? OP_B_out : 24'd0;
    assign OVERFLOW__go = G_fsm__0;
    assign OVERFLOW_left =
     G_fsm__0 ? NOT_ZERO_out : 1'd0;
    assign OVERFLOW_right =
     G_fsm__0 ? OVERFLOW_1_out : 1'd0;
    assign OVERFLOW_1__go = G_fsm__0;
    assign OVERFLOW_1_left =
     G_fsm__0 ? EXP_8_out : 1'd0;
    assign OVERFLOW_1_right =
     G_fsm__0 ? NOT_EXP_7_out : 1'd0;
    assign OVER_MAX__go = G_fsm__0;
    assign OVER_RES_clk = clk;
    assign OVER_RES_exp =
     G_fsm__0 ? OVER_MAX_out : 8'd0;
    assign OVER_RES_go = G_fsm__0;
    assign OVER_RES_mant =
     G_fsm__0 ? ZERO_23_out : 23'd0;
    assign OVER_RES_reset = reset;
    assign OVER_RES_sign =
     G_fsm__0 ? SIGN_out : 1'd0;
    assign PROD22_0__go = G_fsm__0;
    assign PROD22_0_in =
     G_fsm__0 ? PRODUCT_NORMALIZED_out : 48'd0;
    assign PROD46_24__go = G_fsm__0;
    assign PROD46_24_in =
     G_fsm__0 ? PRODUCT_NORMALIZED_out : 48'd0;
    assign PRODUCT__go = G_fsm__0;
    assign PRODUCT_left =
     G_fsm__0 ? OP_A_EXT_out : 48'd0;
    assign PRODUCT_right =
     G_fsm__0 ? OP_B_EXT_out : 48'd0;
    assign PRODUCT_MANTISSA__go = G_fsm__0;
    assign PRODUCT_MANTISSA_left =
     G_fsm__0 ? PROD46_24_out : 23'd0;
    assign PRODUCT_MANTISSA_right =
     G_fsm__0 ? ROUND_SUM_EXT_out : 23'd0;
    assign PRODUCT_NORMALIZED__go = G_fsm__0;
    assign PRODUCT_NORMALIZED_in0 =
     G_fsm__0 ? PRODUCT_out : 48'd0;
    assign PRODUCT_NORMALIZED_in1 =
     G_fsm__0 ? PROD_SHL_out : 48'd0;
    assign PRODUCT_NORMALIZED_sel =
     G_fsm__0 ? NORMALIZED_out : 1'd0;
    assign PROD_MANT_IS_ZERO__go = G_fsm__0;
    assign PROD_MANT_IS_ZERO_left =
     G_fsm__0 ? PRODUCT_MANTISSA_out : 23'd0;
    assign PROD_MANT_IS_ZERO_right =
     G_fsm__0 ? ZERO_23_out : 23'd0;
    assign PROD_NORM_23__go = G_fsm__0;
    assign PROD_NORM_23_in =
     G_fsm__0 ? PRODUCT_NORMALIZED_out : 48'd0;
    assign PROD_SHL__go = G_fsm__0;
    assign PROD_SHL_in =
     G_fsm__0 ? PRODUCT_out : 48'd0;
    assign PROD_SHL_shift =
     G_fsm__0 ? ONE_48_out : 48'd0;
    assign RES__go = G_fsm__0;
    assign RES_in0 =
     G_fsm__0 ? ZERO_32_out : 32'd0;
    assign RES_in1 =
     G_fsm__0 ? RES_3_out : 32'd0;
    assign RES_sel =
     G_fsm__0 ? EXCEPTION_out : 1'd0;
    assign RES_1__go = G_fsm__0;
    assign RES_1_in0 =
     G_fsm__0 ? UNDER_RES_out : 32'd0;
    assign RES_1_in1 =
     G_fsm__0 ? SAFE_RES_out : 32'd0;
    assign RES_1_sel =
     G_fsm__0 ? UNDERFLOW_out : 1'd0;
    assign RES_2__go = G_fsm__0;
    assign RES_2_in0 =
     G_fsm__0 ? OVER_RES_out : 32'd0;
    assign RES_2_in1 =
     G_fsm__0 ? RES_1_out : 32'd0;
    assign RES_2_sel =
     G_fsm__0 ? OVERFLOW_out : 1'd0;
    assign RES_3__go = G_fsm__0;
    assign RES_3_in0 =
     G_fsm__0 ? UNDER_RES_out : 32'd0;
    assign RES_3_in1 =
     G_fsm__0 ? RES_2_out : 32'd0;
    assign RES_3_sel =
     G_fsm__0 ? ZERO_out : 1'd0;
    assign ROUND__go = G_fsm__0;
    assign ROUND_in =
     G_fsm__0 ? PROD22_0_out : 23'd0;
    assign ROUND_SUM__go = G_fsm__0;
    assign ROUND_SUM_left =
     G_fsm__0 ? ROUND_out : 1'd0;
    assign ROUND_SUM_right =
     G_fsm__0 ? PROD_NORM_23_out : 1'd0;
    assign ROUND_SUM_EXT__go = G_fsm__0;
    assign ROUND_SUM_EXT_in =
     G_fsm__0 ? ROUND_SUM_out : 1'd0;
    assign SAFE_RES_clk = clk;
    assign SAFE_RES_exp =
     G_fsm__0 ? EXP_ROUND_out : 8'd0;
    assign SAFE_RES_go = G_fsm__0;
    assign SAFE_RES_mant =
     G_fsm__0 ? PRODUCT_MANTISSA_out : 23'd0;
    assign SAFE_RES_reset = reset;
    assign SAFE_RES_sign =
     G_fsm__0 ? SIGN_out : 1'd0;
    assign SEL0__go = G_fsm__0;
    assign SEL0_in0 =
     G_fsm__0 ? ONE_1_out : 1'd0;
    assign SEL0_in1 =
     G_fsm__0 ? ZERO_1_out : 1'd0;
    assign SEL0_sel =
     G_fsm__0 ? PROD_MANT_IS_ZERO_out : 1'd0;
    assign SIGN__go = G_fsm__0;
    assign SIGN_left =
     G_fsm__0 ? A31_out : 1'd0;
    assign SIGN_right =
     G_fsm__0 ? B31_out : 1'd0;
    assign SUB_V__go = G_fsm__0;
    assign SUM_EXPONENT__go = G_fsm__0;
    assign SUM_EXPONENT_left =
     G_fsm__0 ? EXP_A_out : 8'd0;
    assign SUM_EXPONENT_right =
     G_fsm__0 ? EXP_B_out : 8'd0;
    assign UNDERFLOW__go = G_fsm__0;
    assign UNDERFLOW_left =
     G_fsm__0 ? NOT_ZERO_out : 1'd0;
    assign UNDERFLOW_right =
     G_fsm__0 ? UNDERFLOW_1_out : 1'd0;
    assign UNDERFLOW_1__go = G_fsm__0;
    assign UNDERFLOW_1_left =
     G_fsm__0 ? EXP_7_out : 1'd0;
    assign UNDERFLOW_1_right =
     G_fsm__0 ? EXP_8_out : 1'd0;
    assign UNDER_RES__go = G_fsm__0;
    assign UNDER_RES_left =
     G_fsm__0 ? SIGN_out : 1'd0;
    assign UNDER_RES_right =
     G_fsm__0 ? ZERO_31_out : 31'd0;
    assign ZERO__go = G_fsm__0;
    assign ZERO_in0 =
     G_fsm__0 ? ZERO_1_out : 1'd0;
    assign ZERO_in1 =
     G_fsm__0 ? SEL0_out : 1'd0;
    assign ZERO_sel =
     G_fsm__0 ? EXCEPTION_out : 1'd0;
    assign ZERO_1__go = G_fsm__0;
    assign ZERO_23__go = G_fsm__0;
    assign ZERO_31__go = G_fsm__0;
    assign ZERO_32__go = G_fsm__0;
    assign out = RES_out;
    // COMPONENT END: FP_Mult
endmodule

module main (
    input logic [31:0] left,
    input logic [31:0] right,
    input logic [31:0] res,
    input logic _go,
    input logic reset,
    input logic clk,
    output logic [31:0] gold,
    output logic [31:0] out,
    output logic [31:0] verilog_nopipe,
    output logic [31:0] verilog_pipe
);
    // COMPONENT START: main
    logic G_fsm_clk;
    logic G_fsm_reset;
    logic G_fsm_go;
    logic G_fsm__0;
    logic G_fsm__1;
    logic G_fsm__2;
    logic [31:0] NP_a;
    logic [31:0] NP_b;
    logic NP__go;
    logic NP_exception;
    logic NP_overflow;
    logic NP_underflow;
    logic [31:0] NP_res;
    logic [31:0] P_A;
    logic [31:0] P_B;
    logic P__go;
    logic P_clk;
    logic [31:0] P_res;
    logic [31:0] F_a;
    logic [31:0] F_b;
    logic F_go;
    logic F_reset;
    logic F_clk;
    logic F_exception;
    logic F_overflow;
    logic F_underflow;
    logic [31:0] F_out;
    initial begin
        G_fsm_clk = 1'd0;
        G_fsm_reset = 1'd0;
        G_fsm_go = 1'd0;
        NP_a = 32'd0;
        NP_b = 32'd0;
        NP__go = 1'd0;
        P_A = 32'd0;
        P_B = 32'd0;
        P__go = 1'd0;
        P_clk = 1'd0;
        F_a = 32'd0;
        F_b = 32'd0;
        F_go = 1'd0;
        F_reset = 1'd0;
        F_clk = 1'd0;
    end
    fsm_3 G_fsm (
        ._0(G_fsm__0),
        ._1(G_fsm__1),
        ._2(G_fsm__2),
        .clk(G_fsm_clk),
        .go(G_fsm_go),
        .reset(G_fsm_reset)
    );
    FP_Mult_NoPipe NP (
        ._go(NP__go),
        .a(NP_a),
        .b(NP_b),
        .exception(NP_exception),
        .overflow(NP_overflow),
        .res(NP_res),
        .underflow(NP_underflow)
    );
    FP_Mult_Pipe P (
        .A(P_A),
        .B(P_B),
        ._go(P__go),
        .clk(P_clk),
        .res(P_res)
    );
    FP_Mult F (
        .a(F_a),
        .b(F_b),
        .clk(F_clk),
        .exception(F_exception),
        .go(F_go),
        .out(F_out),
        .overflow(F_overflow),
        .reset(F_reset),
        .underflow(F_underflow)
    );
    assign F_a =
     G_fsm__0 ? left : 32'd0;
    assign F_b =
     G_fsm__0 ? right : 32'd0;
    assign F_clk = clk;
    assign F_go = G_fsm__0;
    assign F_reset = reset;
    assign G_fsm_clk = clk;
    assign G_fsm_go = _go;
    assign G_fsm_reset = reset;
    assign NP__go = G_fsm__0;
    assign NP_a =
     G_fsm__0 ? left : 32'd0;
    assign NP_b =
     G_fsm__0 ? right : 32'd0;
    assign P_A =
     G_fsm__0 ? left : 32'd0;
    assign P_B =
     G_fsm__0 ? right : 32'd0;
    assign P__go = G_fsm__0;
    assign P_clk = clk;
    assign gold = res;
    assign out = F_out;
    assign verilog_nopipe = NP_res;
    assign verilog_pipe = P_res;
    // COMPONENT END: main
endmodule

module fsm_3 (
    input logic clk,
    input logic reset,
    input logic go,
    output logic _0,
    output logic _1,
    output logic _2
);
    // COMPONENT START: fsm_3
    logic r_in;
    logic r_write_en;
    logic r_clk;
    logic r_reset;
    logic r_out;
    logic r_done;
    logic r0_in;
    logic r0_write_en;
    logic r0_clk;
    logic r0_reset;
    logic r0_out;
    logic r0_done;
    initial begin
        r_in = 1'd0;
        r_write_en = 1'd0;
        r_clk = 1'd0;
        r_reset = 1'd0;
        r0_in = 1'd0;
        r0_write_en = 1'd0;
        r0_clk = 1'd0;
        r0_reset = 1'd0;
    end
    std_reg # (
        .WIDTH(1)
    ) r (
        .clk(r_clk),
        .done(r_done),
        .in(r_in),
        .out(r_out),
        .reset(r_reset),
        .write_en(r_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) r0 (
        .clk(r0_clk),
        .done(r0_done),
        .in(r0_in),
        .out(r0_out),
        .reset(r0_reset),
        .write_en(r0_write_en)
    );
    assign _0 = go;
    assign _1 = r_out;
    assign _2 = r0_out;
    assign r_clk = clk;
    assign r_in = go;
    assign r_reset = reset;
    assign r_write_en = 1'd1;
    assign r0_clk = clk;
    assign r0_in = r_out;
    assign r0_reset = reset;
    assign r0_write_en = 1'd1;
    // COMPONENT END: fsm_3
endmodule

module fsm_1 (
    input logic clk,
    input logic reset,
    input logic go,
    output logic _0
);
    // COMPONENT START: fsm_1

    assign _0 = go;
    // COMPONENT END: fsm_1
endmodule
