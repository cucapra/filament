// Latency-Insensitive wrapper for floating point adder with ready-valid interface
module FP_Adder_LI_Wrapper #(
    parameter STAGES = 1  // 1, 2, 3, or 4 stages
) (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] a,
    input [31:0] b,
    input valid_in,
    output ready_out,
    
    // Output interface  
    output [31:0] result,
    output valid_out,
    input ready_in
);

// Internal FIFO to buffer inputs and track pipeline
localparam FIFO_SIZE = (STAGES < 2) ? 4 : (2*STAGES);
localparam ADDR_WIDTH = (FIFO_SIZE <= 4) ? 2 : 3;  // 2 bits for size ≤4, 3 bits for size ≤8
reg [63:0] input_fifo [0:FIFO_SIZE-1];   // Store {a, b}
reg [STAGES:0] valid_shift;              // Track valid data through pipeline
reg [ADDR_WIDTH-1:0] fifo_head, fifo_tail;
reg [ADDR_WIDTH:0] fifo_count;           // One extra bit to count up to FIFO_SIZE

// Core adder signals
reg [31:0] core_a, core_b;
wire [31:0] core_result;

// Output valid persistence
reg output_valid_reg;
wire transaction_complete = output_valid_reg && ready_in;

// Stall logic with proper backpressure
wire output_stalled = output_valid_reg && !ready_in;
wire pipeline_advance = !output_stalled;
wire input_ready = (fifo_count < FIFO_SIZE) && !output_stalled && !reset;

assign ready_out = input_ready;
assign valid_out = output_valid_reg;
assign result = core_result;

integer i;

// Instantiate the core adder
FP_Adder_Wrapper #(.STAGES(STAGES)) core_adder (
    .clk(clk),
    .reset(reset),
    .a(core_a),
    .b(core_b),
    .result(core_result)
);

always @(posedge clk) begin
    if (reset) begin
        fifo_head <= 0;
        fifo_tail <= 0;
        fifo_count <= 0;
        valid_shift <= 0;
        output_valid_reg <= 0;
        core_a <= 0;
        core_b <= 0;
        
        // Clear FIFO
        for (i = 0; i < FIFO_SIZE; i++) begin
            input_fifo[i] <= 64'h0;
        end
    end else begin
        // Input side: enqueue when valid input and we're ready
        if (valid_in && ready_out) begin
            input_fifo[fifo_tail] <= {a, b};
            fifo_tail <= (fifo_tail + 1'b1) % ADDR_WIDTH'(FIFO_SIZE);
            fifo_count <= fifo_count + 1'b1;
        end
        
        // Output valid management - latch valid until transaction completes
        if (valid_shift[STAGES] && !output_valid_reg) begin
            output_valid_reg <= 1'b1;  // Latch valid when data reaches output
        end else if (transaction_complete) begin
            output_valid_reg <= 1'b0;  // Clear after handshake
        end
        
        // Pipeline advance logic - only when not stalled
        if (pipeline_advance) begin
            // Shift valid bits through pipeline
            for (i = STAGES; i > 0; i--) begin
                valid_shift[i] <= valid_shift[i-1];
            end
            
            // Feed new data to adder if FIFO has data
            if (fifo_count > 0) begin
                {core_a, core_b} <= input_fifo[fifo_head];
                fifo_head <= (fifo_head + 1'b1) % ADDR_WIDTH'(FIFO_SIZE);
                fifo_count <= fifo_count - 1'b1;
                valid_shift[0] <= 1'b1;
            end else begin
                valid_shift[0] <= 1'b0;
            end
        end
    end
end

endmodule