# Build and run serial port test with useful arguments
#
#
PORT_1=/dev/tty.usbserial-FT7HED56

PORT_2=/dev/tty.usbserial-FT75LVFT
PORT_2=/dev/tty.usbserial-FT7HE4P6
PORT_2=/dev/tty.usbserial-FT7HE0KL 
PORT_2=/dev/tty.usbserial-FT7HEBRD
PORT_2=/dev/tty.usbserial-FT7HE4YX


#RUST_LOG=debug cargo run --release  -- -b 921600  -f /dev/tty.usbserial-FT75LVFT -s /dev/tty.usbserial-FT7HED56

RUST_LOG=debug cargo run --release  -- -b 921600  -f /dev/tty.usbserial-FT7HE4P6 -s /dev/tty.usbserial-FT7HED56

#RUST_LOG=debug cargo run --release  -- -b 921600  -f /dev/tty.usbserial-FT7HE0KL  -s /dev/tty.usbserial-FT7HED56

#RUST_LOG=debug cargo run --release  -- -b 921600  -f /dev/tty.usbserial-FT7HE4YX   -s /dev/tty.usbserial-FT7HED56

#RUST_LOG=debug cargo run --release  -- -b 921600  -f /dev/tty.usbserial-FT7HEBRD   -s /dev/tty.usbserial-FT7HED56

