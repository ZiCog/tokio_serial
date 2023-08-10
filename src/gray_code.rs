// Exercising the 360 degree 9 sensor single track gray code found here:
// https://www.experts-exchange.com/questions/23594359/%27single-track-gray-code%27-sought-for-encoding-360-degrees-with-9-sensors.html

use bitintr::Popcnt;

pub fn single_track_gray_code() {
    // A single track Gray code with 9 sensors spaced at 40 degreees, 1 degre steps.
    const SingleTrack: &str= "001100000000000000000011111100111111100000011111000000011111000111111110011100000000000111100111001111110000011111100000000000000011110001111111111111111100000000000000000011111111100001100000000000000000000000000000000111111111111111111100011111000000000000000000000000001111111000000111100000000000000000111111111111111111111111111111111111111111111111111111";

    let single_track = SingleTrack.as_bytes();
    assert_eq!(single_track.len(), 360);

    // Construct sensor output look up table from Gray code sequence.
    let mut output_table: Vec<u64> = vec![];
    for angle in 0..360 {
        let mut output = 0u64;
        for sensor in 0..9 {
            let bit = single_track[(angle + sensor * 40) % 360];
            let bit = if bit == b'1' { 1 } else { 0 };
            output = output | (bit << sensor)
        }
        output_table.push(output);
    }
    // Verify only 1 bit changes for each step of output table and print it
    // Nine bit single track gray code output.
    println!("Angle : Output");
    for a in 0..output_table.len() {
        println!("{:03}   : {:09b}", a, output_table[a]);
        let changed_bits = output_table[a] ^ output_table[(a + 1) % output_table.len()];
        assert_eq!(changed_bits.popcnt(), 1);
    }
    // Verfy no duplicate codes in outout table.
    output_table.sort();
    for a in 0..output_table.len() {
        assert_ne!(output_table[a], output_table[(a + 1) % output_table.len()]);
    }
}
