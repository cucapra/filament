// Add_LI module - Latency Insensitive wrapper for Add module
module Add_LI #(
    parameter WIDTH = 32,
    parameter PIPELINE_STAGES = 1  // Number of pipeline stages for the Add module
) (
    input wire clk,
    input wire reset,

    // Input interface with ready-valid handshaking
    input wire [WIDTH-1:0] a_in,     // IEEE 754 single precision input A
    input wire [WIDTH-1:0] b_in,     // IEEE 754 single precision input B
    input wire valid_in,             // Input data is valid
    output logic ready_out,          // Ready to accept new input

    // Output interface with ready-valid handshaking
    output logic [WIDTH-1:0] sum_out,    // IEEE 754 single precision result
    output logic valid_out,          // Output data is valid
    input wire ready_in              // Downstream is ready to accept result
);

    // Internal state for pipeline management
    typedef enum logic [1:0] {
        IDLE    = 2'b00,
        COMPUTE = 2'b01,
        VALID   = 2'b10
    } state_t;

    state_t current_state, next_state;

    // Internal signals for Add module interface
    logic [WIDTH-1:0] add_a, add_b;
    logic [WIDTH-1:0] add_result;

    // Pipeline registers
    logic [WIDTH-1:0] result_reg;
    logic valid_reg;

    logic [$clog2(PIPELINE_STAGES+1)-1:0] stage_counter;

    // Instantiate the Add module
    Add #(.WIDTH(WIDTH), .PIPELINE_STAGES(PIPELINE_STAGES)) add_inst (
        .clk(clk),
        .a(add_a),
        .b(add_b),
        .sum(add_result)
    );

    // Reusable ready-valid state machine and stage counter
    RV_ReadyValid #(.PIPELINE_STAGES(PIPELINE_STAGES)) rv_rv (
        .clk(clk),
        .reset(reset),
        .valid_in(valid_in),
        .valid_out(valid_out),
        .ready_in(ready_in),
        .ready_out(ready_out),
        .stage_counter(stage_counter),
        .current_state(current_state),
        .next_state(next_state)
    );

    // Sequential logic for state updates
    always_ff @(posedge clk) begin
        if (reset) begin
            current_state <= IDLE;
        end else begin
            current_state <= next_state;
        end
    end

    // Input capture logic
    always_ff @(posedge clk) begin
        if (reset) begin
            add_a <= 32'h00000000;
            add_b <= 32'h00000000;
        end else if (current_state == IDLE && valid_in && ready_out) begin
            // Capture inputs when starting computation
            add_a <= a_in;
            add_b <= b_in;
        end
    end

    // Output management logic
    always_ff @(posedge clk) begin
        if (reset) begin
            result_reg <= 32'h00000000;
            valid_reg <= 1'b0;
        end else begin
            case (current_state)
                IDLE: begin
                    valid_reg <= 1'b0;
                end

                COMPUTE: begin
                    valid_reg <= 1'b0;  // Not valid yet
                end

                VALID: begin
                    if (!valid_reg) begin
                        // First cycle in VALID state - capture result
                        result_reg <= add_result;
                        valid_reg <= 1'b1;
                    end else if (ready_in) begin
                        // Output consumed, clear valid
                        valid_reg <= 1'b0;
                    end
                end

                default: begin
                    valid_reg <= 1'b0;
                end
            endcase
        end
    end

    // Output assignments
    assign sum_out = result_reg;
    assign valid_out = valid_reg;

endmodule

// Mul_LI module - Latency Insensitive wrapper for Mul module
module Mul_LI #(
    parameter WIDTH = 32,
    parameter PIPELINE_STAGES = 1  // Number of pipeline stages for the Mul module
) (
    input wire clk,
    input wire reset,

    // Input interface with ready-valid handshaking
    input wire [WIDTH-1:0] a_in,     // IEEE 754 single precision input A
    input wire [WIDTH-1:0] b_in,     // IEEE 754 single precision input B
    input wire valid_in,             // Input data is valid
    output logic ready_out,          // Ready to accept new input

    // Output interface with ready-valid handshaking
    output logic [WIDTH-1:0] product_out, // IEEE 754 single precision result
    output logic valid_out,          // Output data is valid
    input wire ready_in              // Downstream is ready to accept result
);

    // Internal state for pipeline management
    typedef enum logic [1:0] {
        IDLE    = 2'b00,
        COMPUTE = 2'b01,
        VALID   = 2'b10
    } state_t;

    state_t current_state, next_state;

    // Internal signals for Mul module interface
    logic [WIDTH-1:0] mul_a, mul_b;
    logic [WIDTH-1:0] mul_result;

    // Pipeline registers
    logic [WIDTH-1:0] result_reg;
    logic valid_reg;

    logic [$clog2(PIPELINE_STAGES+1)-1:0] stage_counter;

    // Instantiate the Mul module
    Mul #(.WIDTH(WIDTH), .PIPELINE_STAGES(PIPELINE_STAGES)) mul_inst (
        .clk(clk),
        .a(mul_a),
        .b(mul_b),
        .product(mul_result)
    );

    // Reusable ready-valid state machine and stage counter
    RV_ReadyValid #(.PIPELINE_STAGES(PIPELINE_STAGES)) rv_rv (
        .clk(clk),
        .reset(reset),
        .valid_in(valid_in),
        .valid_out(valid_out),
        .ready_in(ready_in),
        .ready_out(ready_out),
        .stage_counter(stage_counter),
        .current_state(current_state),
        .next_state(next_state)
    );

    // Sequential logic for state updates
    always_ff @(posedge clk) begin
        if (reset) begin
            current_state <= IDLE;
        end else begin
            current_state <= next_state;
        end
    end

    // Input capture logic
    always_ff @(posedge clk) begin
        if (reset) begin
            mul_a <= 32'h00000000;
            mul_b <= 32'h00000000;
        end else if (current_state == IDLE && valid_in && ready_out) begin
            // Capture inputs when starting computation
            mul_a <= a_in;
            mul_b <= b_in;
        end
    end

    // Output management logic
    always_ff @(posedge clk) begin
        if (reset) begin
            result_reg <= 32'h00000000;
            valid_reg <= 1'b0;
        end else begin
            case (current_state)
                IDLE: begin
                    valid_reg <= 1'b0;
                end

                COMPUTE: begin
                    valid_reg <= 1'b0;  // Not valid yet
                end

                VALID: begin
                    if (!valid_reg) begin
                        // First cycle in VALID state - capture result
                        result_reg <= mul_result;
                        valid_reg <= 1'b1;
                    end else if (ready_in) begin
                        // Output consumed, clear valid
                        valid_reg <= 1'b0;
                    end
                end

                default: begin
                    valid_reg <= 1'b0;
                end
            endcase
        end
    end

    // Output assignments
    assign product_out = result_reg;
    assign valid_out = valid_reg;

endmodule

// ALU_LI module - Latency Insensitive ALU using Add_LI and Mul_LI modules
module ALU_LI #(
    parameter WIDTH = 32,
    parameter ADD_S = 1,  // Number of pipeline stages for adder
    parameter MUL_S = 1   // Number of pipeline stages for multiplier
) (
    input wire clk,
    input wire reset,

    // Input interface with ready-valid handshaking
    input wire [WIDTH-1:0] a_in,     // IEEE 754 single precision input A
    input wire [WIDTH-1:0] b_in,     // IEEE 754 single precision input B
    input wire op_in,                // 0 for add, 1 for multiply
    input wire valid_in,             // Input data is valid
    output logic ready_out,          // Ready to accept new input

    // Output interface with ready-valid handshaking
    output logic [WIDTH-1:0] result_out, // IEEE 754 single precision result
    output logic valid_out,          // Output data is valid
    input wire ready_in              // Downstream is ready to accept result
);

    // Operation selection
    typedef enum logic {
        OP_ADD = 1'b0,
        OP_MUL = 1'b1
    } alu_op_t;

    // Internal signals for Add_LI module
    logic [WIDTH-1:0] add_sum;
    logic add_valid_out;
    logic add_ready_out;
    logic add_ready_in;

    // Internal signals for Mul_LI module
    logic [WIDTH-1:0] mul_product;
    logic mul_valid_out;
    logic mul_ready_out;
    logic mul_ready_in;

    // Operation capture
    logic captured_op;
    logic op_valid;

    // Instantiate Add_LI module
    Add_LI #(.WIDTH(WIDTH), .PIPELINE_STAGES(ADD_S)) add_li_inst (
        .clk(clk),
        .reset(reset),
        .a_in(a_in),
        .b_in(b_in),
        .valid_in(valid_in && (op_in == OP_ADD)),
        .ready_out(add_ready_out),
        .sum_out(add_sum),
        .valid_out(add_valid_out),
        .ready_in(add_ready_in)
    );

    // Instantiate Mul_LI module
    Mul_LI #(.WIDTH(WIDTH), .PIPELINE_STAGES(MUL_S)) mul_li_inst (
        .clk(clk),
        .reset(reset),
        .a_in(a_in),
        .b_in(b_in),
        .valid_in(valid_in && (op_in == OP_MUL)),
        .ready_out(mul_ready_out),
        .product_out(mul_product),
        .valid_out(mul_valid_out),
        .ready_in(mul_ready_in)
    );

    // Capture operation when input is accepted
    always_ff @(posedge clk) begin
        if (reset) begin
            captured_op <= 1'b0;
            op_valid <= 1'b0;
        end else if (valid_in && ready_out) begin
            captured_op <= op_in;
            op_valid <= 1'b1;
        end else if (valid_out && ready_in) begin
            op_valid <= 1'b0;
        end
    end

    // Output multiplexing based on captured operation
    always_comb begin
        case (captured_op)
            OP_ADD: begin
                result_out = add_sum;
                valid_out = add_valid_out && op_valid;
                add_ready_in = ready_in;
                mul_ready_in = 1'b1;  // Always ready to discard mul result
            end
            OP_MUL: begin
                result_out = mul_product;
                valid_out = mul_valid_out && op_valid;
                mul_ready_in = ready_in;
                add_ready_in = 1'b1;  // Always ready to discard add result
            end
            default: begin
                result_out = 32'h00000000;
                valid_out = 1'b0;
                add_ready_in = 1'b1;
                mul_ready_in = 1'b1;
            end
        endcase
    end

    // Ready output - ready when the selected operation unit is ready
    always_comb begin
        case (op_in)
            OP_ADD: ready_out = add_ready_out;
            OP_MUL: ready_out = mul_ready_out;
            default: ready_out = 1'b0;
        endcase
    end

endmodule