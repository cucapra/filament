// Add module - performs floating-point addition in one cycle
module Add #(
    parameter WIDTH = 32
) (
    input wire clk,
    input wire [WIDTH-1:0] a,  // IEEE 754 single precision
    input wire [WIDTH-1:0] b,  // IEEE 754 single precision
    output reg [WIDTH-1:0] sum // IEEE 754 single precision result
);
    // Extract fields for floating-point addition
    wire sign_a, sign_b;
    wire [7:0] exp_a, exp_b;
    wire [22:0] mant_a, mant_b;
    wire [23:0] full_mant_a, full_mant_b; // Include implicit leading 1

    // Intermediate signals for combinational logic
    logic [7:0] exp_diff;
    logic [24:0] aligned_mant_a, aligned_mant_b;
    logic [24:0] mant_sum;
    logic [7:0] result_exp;
    logic result_sign;
    logic [22:0] result_mant;
    logic [WIDTH-1:0] sum_next;
    logic is_zero_a, is_zero_b;

    // Field extraction
    assign sign_a = a[31];
    assign exp_a = a[30:23];
    assign mant_a = a[22:0];
    assign sign_b = b[31];
    assign exp_b = b[30:23];
    assign mant_b = b[22:0];

    // Add implicit leading 1 for normalized numbers
    assign full_mant_a = (exp_a == 8'b0) ? {1'b0, mant_a} : {1'b1, mant_a};
    assign full_mant_b = (exp_b == 8'b0) ? {1'b0, mant_b} : {1'b1, mant_b};

    // Zero detection
    assign is_zero_a = (exp_a == 8'b0) && (mant_a == 23'b0);
    assign is_zero_b = (exp_b == 8'b0) && (mant_b == 23'b0);

    // Combinational logic for floating-point addition
    always_comb begin
        // Default values
        exp_diff = 8'b0;
        aligned_mant_a = 25'b0;
        aligned_mant_b = 25'b0;
        mant_sum = 25'b0;
        result_exp = 8'b0;
        result_sign = 1'b0;
        result_mant = 23'b0;
        sum_next = 32'h00000000;

        // Handle special cases (zero inputs)
        if (is_zero_a) begin
            sum_next = b;
        end else if (is_zero_b) begin
            sum_next = a;
        end else begin
            // Determine larger exponent and align mantissas
            if (exp_a >= exp_b) begin
                result_exp = exp_a;
                exp_diff = exp_a - exp_b;
                aligned_mant_a = {full_mant_a, 1'b0};
                aligned_mant_b = {full_mant_b, 1'b0} >> exp_diff;
            end else begin
                result_exp = exp_b;
                exp_diff = exp_b - exp_a;
                aligned_mant_a = {full_mant_a, 1'b0} >> exp_diff;
                aligned_mant_b = {full_mant_b, 1'b0};
            end

            // Add or subtract based on signs
            if (sign_a == sign_b) begin
                mant_sum = aligned_mant_a + aligned_mant_b;
                result_sign = sign_a;
            end else begin
                if (aligned_mant_a >= aligned_mant_b) begin
                    mant_sum = aligned_mant_a - aligned_mant_b;
                    result_sign = sign_a;
                end else begin
                    mant_sum = aligned_mant_b - aligned_mant_a;
                    result_sign = sign_b;
                end
            end

            // Normalize result
            if (mant_sum[24]) begin
                // Overflow, shift right and increment exponent
                result_mant = mant_sum[23:1];
                result_exp = result_exp + 1;
            end else if (mant_sum[23]) begin
                // Normal case
                result_mant = mant_sum[22:0];
            end else begin
                // Need to shift left (simplified - assumes at most 1 shift needed)
                result_mant = mant_sum[21:0] << 1;
                result_exp = result_exp - 1;
            end

            sum_next = {result_sign, result_exp, result_mant};
        end
    end

    // Sequential logic - register the output
    always_ff @(posedge clk) begin
        sum <= sum_next;
    end
endmodule

// Mul module - performs floating-point multiplication in one cycle
module Mul #(
    parameter WIDTH = 32
) (
    input wire clk,
    input wire [WIDTH-1:0] a,       // IEEE 754 single precision
    input wire [WIDTH-1:0] b,       // IEEE 754 single precision
    output reg [WIDTH-1:0] product  // IEEE 754 single precision result
);
    // Extract fields for floating-point multiplication
    wire sign_a, sign_b;
    wire [7:0] exp_a, exp_b;
    wire [22:0] mant_a, mant_b;
    wire [23:0] full_mant_a, full_mant_b; // Include implicit leading 1

    // Intermediate signals for combinational logic
    logic [47:0] mant_product;
    logic [8:0] exp_sum; // 9 bits to handle overflow
    logic result_sign;
    logic [7:0] result_exp;
    logic [22:0] result_mant;
    logic [WIDTH-1:0] product_next;
    logic is_zero_a, is_zero_b;
    logic exp_overflow, exp_underflow;

    // Field extraction
    assign sign_a = a[31];
    assign exp_a = a[30:23];
    assign mant_a = a[22:0];
    assign sign_b = b[31];
    assign exp_b = b[30:23];
    assign mant_b = b[22:0];

    // Add implicit leading 1 for normalized numbers
    assign full_mant_a = (exp_a == 8'b0) ? {1'b0, mant_a} : {1'b1, mant_a};
    assign full_mant_b = (exp_b == 8'b0) ? {1'b0, mant_b} : {1'b1, mant_b};

    // Zero detection
    assign is_zero_a = (exp_a == 8'b0) && (mant_a == 23'b0);
    assign is_zero_b = (exp_b == 8'b0) && (mant_b == 23'b0);

    // Combinational logic for floating-point multiplication
    always_comb begin
        // Default values
        mant_product = 48'b0;
        exp_sum = 9'b0;
        result_sign = 1'b0;
        result_exp = 8'b0;
        result_mant = 23'b0;
        product_next = 32'h00000000;
        exp_overflow = 1'b0;
        exp_underflow = 1'b0;

        // Handle special cases (zero inputs)
        if (is_zero_a || is_zero_b) begin
            product_next = 32'h00000000; // Result is zero
        end else begin
            // Calculate result sign
            result_sign = sign_a ^ sign_b;

            // Multiply mantissas
            mant_product = full_mant_a * full_mant_b;

            // Add exponents and subtract bias (127)
            exp_sum = exp_a + exp_b - 127;

            // Check for overflow/underflow
            exp_overflow = exp_sum[8] || (exp_sum > 254);
            exp_underflow = exp_sum < 1;

            // Normalize result
            if (mant_product[47]) begin
                // MSB is 1, shift right and increment exponent
                result_mant = mant_product[46:24];
                result_exp = (exp_sum + 1 > 255) ? 8'hFF : exp_sum[7:0] + 1;
            end else begin
                // MSB is 0, use as is
                result_mant = mant_product[45:23];
                result_exp = (exp_sum > 255) ? 8'hFF : exp_sum[7:0];
            end

            // Handle overflow/underflow cases
            if (exp_overflow) begin
                // Overflow - return infinity
                product_next = {result_sign, 8'hFF, 23'b0};
            end else if (exp_underflow) begin
                // Underflow - return zero
                product_next = {result_sign, 8'h00, 23'b0};
            end else begin
                product_next = {result_sign, result_exp, result_mant};
            end
        end
    end

    // Sequential logic - register the output
    always_ff @(posedge clk) begin
        product <= product_next;
    end
endmodule
