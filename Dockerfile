# AI Generated File
# Use the official Rust image as the base image
FROM rust:latest

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the local Cargo.toml and Cargo.lock files into the container
COPY Cargo.toml Cargo.lock ./

# Build dependencies (this step is separate to leverage Docker cache)
RUN cargo build --release

# Copy the rest of the application code into the container
COPY . .

# Build the application with a generic executable name
ARG EXEC_NAME=my_executable

RUN cargo build --release --features "$EXEC_NAME"

# Set the command to run your application
CMD ["./target/release/my_executable"]