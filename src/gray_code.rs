// Exercising the 360 degree 9 sensor single track gray code found here:
// https://www.experts-exchange.com/questions/23594359/%27single-track-gray-code%27-sought-for-encoding-360-degrees-with-9-sensors.html

use bitintr::Popcnt;

pub fn single_track_gray_code() {
    // A single track Gray code with 9 sensors spaced at 40 degreees, 1 degre steps.
    const SingleTrack: &str= "001100000000000000000011111100111111100000011111000000011111000111111110011100000000000111100111001111110000011111100000000000000011110001111111111111111100000000000000000011111111100001100000000000000000000000000000000111111111111111111100011111000000000000000000000000001111111000000111100000000000000000111111111111111111111111111111111111111111111111111111";
    const TrackLength: usize = 360;
    let single_track = SingleTrack.as_bytes();
    assert_eq!(single_track.len(), TrackLength);

    // Construct sensor output look up table from Gray code sequence.
    let mut output_table: Vec<u16> = vec![];
    for angle in 0..TrackLength {
        let mut output = 0u16;
        for sensor in 0..9 {
            let bit = single_track[(angle + sensor * 40) % TrackLength];
            let bit = if bit == b'1' { 1 } else { 0 };
            output = output | (bit << sensor)
        }
        output_table.push(output);
    }
    // Verify only 1 bit changes for each step of output table and print
    for a in 0..output_table.len() {
        let changed_bits = output_table[a] ^ output_table[(a + 1) % TrackLength];
        assert_eq!(changed_bits.popcnt(), 1);
    }

    // Generate the input table (revese look up, sensor to angle)
    let mut input_table: Vec<Option<u16>> = vec![None; 512];
    for a in 0..output_table.len() {
        input_table[output_table[a as usize] as usize] = Some(a as u16);
    }
    // Print the ouput table
    println!("Angle : output");
    for a in 0..output_table.len() {
        println!("{} : {:?}", a, output_table[a]);
    }

    // Print the input table
    println!("Input : angle");
    for i in 0..input_table.len() {
        println!("{} : {:?}", i, input_table[i]);
    }

    // Verfy no duplicate codes in outout table.
    output_table.sort();
    for a in 0..output_table.len() {
        assert_ne!(output_table[a], output_table[(a + 1) % TrackLength]);
    }
}
