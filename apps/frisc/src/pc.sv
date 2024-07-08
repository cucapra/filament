module PC (
  input wire clk,
  input wire reset,
  input logic [31:0] nextpc,
  output logic [31:0] pc
);
  always_ff @(posedge clk) begin
    if (reset) begin 
      pc <= 32'd0;
    end
    else begin 
      pc <= nextpc;
    end
  end
endmodule
