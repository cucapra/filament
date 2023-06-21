module StdMem1D #(
    parameter DATA_WIDTH = 32,
    parameter SIZE = 16,
    parameter ADDR_WIDTH = 4
) (
   input wire                logic [ADDR_WIDTH-1:0] addr0,
   input wire                logic [ DATA_WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   input wire                logic reset,
   output logic [ DATA_WIDTH-1:0] read_data,
   output logic              done
);

  logic [DATA_WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];

  // always_ff @(posedge clk) begin
  //   if (reset)
  //     done <= '0;
  //   else if (write_en)
  //     done <= '1;
  //   else
  //     done <= '0;
  // end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0] <= write_data;
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