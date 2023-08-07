fn main() {
    protobuf_codegen::Codegen::new()
        .cargo_out_dir("protos")
        .include("src")
        .input("src/protos/example.proto")
        .run_from_script();

    cc::Build::new().file("src/hdlc.c").compile("hdlc");
}
