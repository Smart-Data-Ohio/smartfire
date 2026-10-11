# Soundboard media fixtures

These 0.1-second mono clips were generated with local FFmpeg 9.0.2.
Run these commands in this directory with a full FFmpeg build to regenerate them.
The tests load the bytes directly and use ffprobe for analysis.

```sh
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i anullsrc=r=48000:cl=mono -t 0.1 -map_metadata -1 -c:a libmp3lame -b:a 32k -write_xing 0 -id3v2_version 3 -write_id3v1 0 audio.mp3
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i anullsrc=r=48000:cl=mono -t 0.1 -map_metadata -1 -c:a libopus -b:a 8k audio.ogg
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i anullsrc=r=48000:cl=mono -t 0.1 -map_metadata -1 -c:a libopus -b:a 8k audio.webm
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i anullsrc=r=48000:cl=mono -t 0.1 -map_metadata -1 -c:a flac -metadata_header_padding 0 audio.flac
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i anullsrc=r=48000:cl=mono -f lavfi -i color=c=black:s=16x16:r=10 -map 0:a -map 1:v -t 0.1 -map_metadata -1 -c:a libmp3lame -b:a 32k -write_xing 0 -id3v2_version 3 -write_id3v1 0 -c:v mjpeg -disposition:v attached_pic -frames:v 1 artwork.mp3
ffmpeg -v error -threads 1 -filter_threads 1 -f lavfi -i color=c=black:s=16x16:r=10 -f lavfi -i anullsrc=r=48000:cl=mono -t 0.1 -map_metadata -1 -c:v libvpx -b:v 8k -c:a libopus -b:a 8k video.webm
```

The tests strip the ID3v2 tag from `audio.mp3` for frame-sync coverage and prepend
an empty ID3v2 header to the FLAC and Ogg bytes for MIME mismatch coverage.
WAV coverage uses the existing `../analysis.wav` fixture.
