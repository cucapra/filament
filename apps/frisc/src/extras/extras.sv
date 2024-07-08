module OneHotMux #(
  parameter WIDTH = 32
) (
  input wire logic [7:0] sel,
  input wire logic [WIDTH-1:0] in0,
  input wire logic [WIDTH-1:0] in1,
  input wire logic [WIDTH-1:0] in2,
  input wire logic [WIDTH-1:0] in3,
  input wire logic [WIDTH-1:0] in4,
  input wire logic [WIDTH-1:0] in5,
  input wire logic [WIDTH-1:0] in6,
  input wire logic [WIDTH-1:0] in7,
  output reg [WIDTH-1:0] out
);
  always @(*) begin
    case(sel)
      8'b00000001: out = in0;
      8'b00000010: out = in1;
      8'b00000100: out = in2;
      8'b00001000: out = in3;
      8'b00010000: out = in4;
      8'b00100000: out = in5;
      8'b01000000: out = in6;
      8'b10000000: out = in7;
      default: out = in0;
    endcase
  end
endmodule

// byte addressable memory
module ByteAccess1D #(
  parameter SIZE = 16,
  parameter ADDR_WIDTH = 32
) (
  input wire logic clk,
  input wire logic reset,
  input wire logic [ADDR_WIDTH-1:0] addr0,
  input wire logic [31:0] write_data,
  input wire logic [3:0] write_mask,
  output logic [31:0] read_data
);

  logic [7:0] mem [SIZE-1:0];
  
  // reads
  assign read_data[7:0] = mem[addr0];
  assign read_data[15:8] = mem[addr0+1];
  assign read_data[23:16] = mem[addr0+2];
  assign read_data[31:24] = mem[addr0+3];

  // writes, byte by byte based on write mask
  always_ff @(posedge clk) begin
    if (write_mask[0]) begin mem[addr0] <= write_data[7:0]; end
    if (write_mask[1]) mem[addr0+1] <= write_data[15:8];
    if (write_mask[2]) mem[addr0+2] <= write_data[23:16];
    if (write_mask[3]) mem[addr0+3] <= write_data[31:24];
  end
endmodule