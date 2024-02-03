use libc;

#[link(name="convert", kind="static")]
extern "C" {
    pub fn convert_video_from_mpeg_to_mp4(input: *const libc::c_char, output: *const libc::c_char) -> libc::c_int;
}
