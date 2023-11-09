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

  // Registers for pipe stage 3:
  reg [31:0] p3_a;
  reg [31:0] p3_b;
  always_ff @ (posedge clk) begin
    p3_a <= p2_a;
    p3_b <= p2_b;
  end

  // ===== Pipe stage 4:

  // Registers for pipe stage 4:
  reg [31:0] p4_a;
  reg [31:0] p4_b;
  always_ff @ (posedge clk) begin
    p4_a <= p3_a;
    p4_b <= p3_b;
  end

  // ===== Pipe stage 5:

  // Registers for pipe stage 5:
  reg [31:0] p5_a;
  reg [31:0] p5_b;
  always_ff @ (posedge clk) begin
    p5_a <= p4_a;
    p5_b <= p4_b;
  end

  // ===== Pipe stage 6:

  // Registers for pipe stage 6:
  reg [31:0] p6_a;
  reg [31:0] p6_b;
  always_ff @ (posedge clk) begin
    p6_a <= p5_a;
    p6_b <= p5_b;
  end

  // ===== Pipe stage 7:

  // Registers for pipe stage 7:
  reg [31:0] p7_a;
  reg [31:0] p7_b;
  always_ff @ (posedge clk) begin
    p7_a <= p6_a;
    p7_b <= p6_b;
  end

  // ===== Pipe stage 8:

  // Registers for pipe stage 8:
  reg [31:0] p8_a;
  reg [31:0] p8_b;
  always_ff @ (posedge clk) begin
    p8_a <= p7_a;
    p8_b <= p7_b;
  end

  // ===== Pipe stage 9:

  // Registers for pipe stage 9:
  reg [31:0] p9_a;
  reg [31:0] p9_b;
  always_ff @ (posedge clk) begin
    p9_a <= p8_a;
    p9_b <= p8_b;
  end

  // ===== Pipe stage 10:
  wire [31:0] p10_umul_46_comb;
  assign p10_umul_46_comb = umul32b_32b_x_32b(p9_a, p9_b);

  // Registers for pipe stage 10:
  reg [31:0] p10_umul_46;
  always_ff @ (posedge clk) begin
    p10_umul_46 <= p10_umul_46_comb;
  end
  assign out = p10_umul_46;
endmodule
