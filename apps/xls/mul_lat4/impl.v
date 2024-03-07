module Mul(
  input wire clk,
  input wire [31:0] a,
  input wire [31:0] b,
  output wire [31:0] out
);
  // lint_off MULTIPLY
  function automatic [31:0] umul32b_32b_x_32b (input reg [31:0] lhs, input reg [31:0] rhs);
    begin
      umul32b_32b_x_32b = lhs * rhs;
    end
  endfunction
  // lint_on MULTIPLY

  // ===== Pipe stage 0:

  // Registers for pipe stage 0:
  reg [31:0] p0_a;
  reg [31:0] p0_b;
  always_ff @ (posedge clk) begin
    p0_a <= a;
    p0_b <= b;
  end

  // ===== Pipe stage 1:

  // Registers for pipe stage 1:
  reg [31:0] p1_a;
  reg [31:0] p1_b;
  always_ff @ (posedge clk) begin
    p1_a <= p0_a;
    p1_b <= p0_b;
  end

  // ===== Pipe stage 2:

  // Registers for pipe stage 2:
  reg [31:0] p2_a;
  reg [31:0] p2_b;
  always_ff @ (posedge clk) begin
    p2_a <= p1_a;
    p2_b <= p1_b;
  end

  // ===== Pipe stage 3:
  wire [31:0] p3_umul_18_comb;
  assign p3_umul_18_comb = umul32b_32b_x_32b(p2_a, p2_b);

  // Registers for pipe stage 3:
  reg [31:0] p3_umul_18;
  always_ff @ (posedge clk) begin
    p3_umul_18 <= p3_umul_18_comb;
  end
  assign out = p3_umul_18;
endmodule
