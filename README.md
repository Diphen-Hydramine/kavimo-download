# Kavimo Download

## Quick start

1) download the latests version from [here](https://github.com/Diphen-Hydramine/kavimo-download/releases/download/v1.0.0/kavimo-download.exe).
2) provide the iframe link to application.
3) have a happy ending :)

All kavimo hosted videoes are inside \<iframe> tags href attribute of those iframes are similar to 

`https://stream.biomaze.ir/b6tnnbbopku1/iframe`

the only thing program needs from you is this link and the rest is managed automatically.

* you should turn off you anti virus software if you want to use the prebuilt version.
* phone number and all other drm_text attributes are removed from the video.

## How does it work?
* This product uses FFmpeg under the hood to convert mpeg stream to mp4 because mpeg streams kinda lag in most video playing software
* The rest is reverse engineered from the Vis2.js Product, a web video player from kavimo

## Notes
* Libraries provided are windows compatible only, there for you should technically run into problems if you try to compile for linux/mac. I'll think for a work around later but now the fastets way to fix this is to remove the `convert_video_from_mpeg_to_mp4` function from the source code compeletly. you may also have to remove links from `build.rs` file.

## Disclaimer

This software is provided for research purposes only. The creators and distributors of this product do not endorse or encourage any illegal activities, including the unauthorized bypassing of digital rights management (DRM) mechanisms. The use of this software to circumvent DRM protection may violate copyright laws and terms of service agreements.

Users are solely responsible for their own actions and must adhere to the laws and regulations of their respective jurisdictions. The creators and distributors of this product shall not be held liable for any legal consequences, damages, or losses resulting from the misuse, abuse, or unauthorized use of this software.

This product is intended for educational and research purposes to foster a better understanding of DRM mechanisms. Users are advised to use this software responsibly, respecting the rights of content creators and intellectual property owners. Any actions taken by users that violate applicable laws or terms of service are entirely their own responsibility.

By downloading, installing, or using this software, users acknowledge and accept the terms of this disclaimer. If in doubt about the legality of specific actions, users are strongly encouraged to seek legal advice before proceeding.
