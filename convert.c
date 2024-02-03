#include <FFmpeg-n6.1/libavformat/avformat.h>
#include <FFmpeg-n6.1/libavcodec/codec_par.h>

int convert_video_from_mpeg_to_mp4(char *input_file, char *output_file) {
    AVFormatContext *inFormatCtx = NULL, *outFormatCtx = NULL;
    avformat_open_input(&inFormatCtx, input_file, NULL, NULL);
    avformat_find_stream_info(inFormatCtx, NULL);
    avformat_alloc_output_context2(&outFormatCtx, NULL, NULL, output_file);
    AVStream *outStream = NULL;
    const AVOutputFormat *outFormat = outFormatCtx->oformat;
    for (int i = 0; i < inFormatCtx->nb_streams; i++) {
        AVCodecParameters *inCodecPar = inFormatCtx->streams[i]->codecpar;
        outStream = avformat_new_stream(outFormatCtx, NULL);
        avcodec_parameters_copy(outStream->codecpar, inCodecPar);
        outStream->codecpar->codec_tag = 0;
    }
    if (!(outFormat->flags & AVFMT_NOFILE)) {
        avio_open(&outFormatCtx->pb, output_file, AVIO_FLAG_WRITE);
    }
    int _ = avformat_write_header(outFormatCtx, NULL);
    AVStream *inStream = NULL;
    AVPacket pkt;
    while (1) {
        int ret_value = av_read_frame(inFormatCtx, &pkt);
        if (ret_value <= -1) {
            break;
        }
        outStream = outFormatCtx->streams[pkt.stream_index];
        inStream  = inFormatCtx->streams[pkt.stream_index];
        pkt.dts = av_rescale_q_rnd(pkt.dts, inStream->time_base, outStream->time_base, AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX);
        pkt.pts = av_rescale_q_rnd(pkt.pts, inStream->time_base, outStream->time_base, AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX);
        pkt.pos = -1;
        pkt.duration = av_rescale_q(pkt.duration, inStream->time_base, outStream->time_base);
        av_interleaved_write_frame(outFormatCtx, &pkt);
        av_packet_unref(&pkt);
    }

    av_write_trailer(outFormatCtx);
    avformat_close_input(&inFormatCtx);
    if (outFormatCtx && !(outFormat->flags & AVFMT_NOFILE)) {
        avio_closep(&outFormatCtx->pb);
    }
    avformat_free_context(outFormatCtx);
    return 0;
}