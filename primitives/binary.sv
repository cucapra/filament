`default_nettype none

module Add (
  input wire logic [31:0] left,
  input wire logic [31:0] right,
  output wire logic [31:0] out
);
  assign out = left + right;
endmodule

module MultComb (
  input wire logic [31:0] left,
  input wire logic [31:0] right,
  output wire logic [31:0] out
);
  assign out = left * right;
endmodule
