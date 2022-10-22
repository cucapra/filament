module Next(
  input logic [8:0] acc,
  input logic [7:0] right,
  input logic [7:0] quotient,
  output logic [8:0] acc_next,
  output logic [7:0] quotient_next
);
  logic [16:0] c;
  logic [8:0] right_ext, sub;
  logic check;

  assign right_ext = {1'b0, right};
  assign check = acc >= right_ext;

  // True branch
  assign sub = acc - right;
  assign c = check ? {sub[7:0], quotient, 1'b1} : ({acc, quotient} << 1);

  assign quotient_next = c[7:0];
  assign acc_next = c[16:8];
endmodule
