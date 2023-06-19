module PC (
  input wire clk,
  input wire reset,
  input logic [31:0] pcin,
  input wire logic isJAL,
  input wire logic isAUIPC,
  input wire logic [31:0] Jimm,
  input wire logic [31:0] Uimm,
  input wire logic [31:0] Bimm,
  output logic [31:0] outImm,
  output logic [31:0] out4
);
  always_ff @(posedge clk) begin
    if (reset) begin 
      out4 <= 32'd0;
      outImm <= 32'd0;
    end
    else begin 
      out4 <= pcin + 32'd4;
      outImm <= pcin + (isJAL ? Jimm : isAUIPC ? Uimm : Bimm);
    end
  end
endmodule