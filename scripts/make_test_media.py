"""Generate spoken fixtures and truth from the same sample clock on macOS."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import wave

fixtures = Path('tests/fixtures')
fixtures.mkdir(parents=True, exist_ok=True)
ffmpeg = os.environ.get("HLP_FFMPEG", "ffmpeg")
speech = {
    'ja': ('Kyoko', '今日はとてもいい天気です。|駅の前で友達と会いました。|私たちは公園まで歩きました。|小さな子供が遊んでいます。|夕方には家に帰ります。|明日もここに来たいです。'),
    'fr': ('Thomas', "Le soleil brille dans le ciel.|Nous allons au marché ce matin.|La petite maison est près de la rivière.|Les enfants jouent dans le jardin.|Je prépare le dîner avec ma famille.|Demain nous irons à la plage."),
    'de': ('Anna', 'Heute scheint die Sonne.|Wir gehen zusammen in den Park.|Der kleine Hund spielt im Garten.|Meine Schwester liest ein Buch.|Am Abend kochen wir zusammen.|Morgen fahren wir mit dem Zug.'),
    'es': ('Mónica', 'Hoy hace muy buen tiempo.|Vamos al mercado por la mañana.|Mi hermana vive cerca del río.|Los niños juegan en el parque.|Esta noche cenamos en casa.|Mañana iremos a la playa.'),
    'zh': ('Tingting', '今天的天气很好。|我们早上一起去公园。|河边有一座小房子。|孩子们在花园里玩耍。|晚上我们一起做饭。|明天我们坐火车回家。'),
    'ko': ('Yuna', '오늘은 날씨가 아주 좋습니다.|우리는 아침에 공원에 갑니다.|동생이 재미있는 책을 읽습니다.|아이들이 정원에서 놀고 있습니다.|저녁에는 가족과 함께 식사를 합니다.|내일은 기차를 타고 집에 갑니다.'),
    'en': ('Samantha', 'Today the sky is clear and blue.|We walk to the station in the morning.|The small house is beside the river.|The children are playing in the garden.|We cook dinner together every evening.|Tomorrow we will take the train home.'),
}
def run(*args):
    subprocess.run([str(a) for a in args], check=True)
def ff(*args):
    run(ffmpeg, '-nostdin', '-hide_banner', '-loglevel', 'error', '-y', *args)
def truth(name, lang, lines):
    (fixtures / (name+'.truth.json')).write_text(json.dumps({'language':lang, 'sentences':lines},ensure_ascii=False,indent=2)+'\n')
formats = {'ja':('mp4','h264_videotoolbox','aac'), 'fr':('mkv','h264_videotoolbox','ac3'), 'de':('webm','libvpx-vp9','libopus'), 'es':('avi','mpeg4','libmp3lame'), 'zh':('mov','hevc_videotoolbox','aac'), 'ko':('mkv','h264_videotoolbox','aac'), 'en':('m4a',None,'aac')}
video = ['-f','lavfi','-i','testsrc2=size=1280x720:rate=30']
with tempfile.TemporaryDirectory() as td:
    tmp=Path(td)
    for lang, (voice, script) in speech.items():
        stem='fixture_'+('ko_51' if lang=='ko' else lang)
        lines=[]
        pcm=bytearray(16000)  # 0.5 seconds of silence
        for sentence in script.split('|'):
            run('say','-v',voice,'-r','160','-o',tmp/'line.aiff',sentence)
            ff('-i',tmp/'line.aiff','-af','silenceremove=start_periods=1:start_threshold=-50dB,areverse,silenceremove=start_periods=1:start_threshold=-50dB,areverse','-ar',16000,'-ac',1,'-f','s16le',tmp/'line.pcm')
            raw=(tmp/'line.pcm').read_bytes()
            if not raw: raise RuntimeError(f'{voice} generated no speech')
            lines.append({'onsetMs':len(pcm)//32, 'text':sentence})
            pcm.extend(raw)
            pcm.extend(bytes(48000))
        with wave.open(str(tmp/'track.wav'),'wb') as wav:
            wav.setparams((1,2,16000,0,'NONE','')); wav.writeframes(pcm)
        ext,vc,ac=formats[lang]
        flags=['-c:v',vc,'-b:v','900k','-pix_fmt','yuv420p'] if vc else []
        if lang=='de': flags+=['-deadline','realtime','-cpu-used','8','-row-mt','1']
        if lang=='zh': flags+=['-tag:v','hvc1']
        if lang=='ko':
            flags=['-f','lavfi','-i','anoisesrc=color=pink:amplitude=0.04:sample_rate=16000','-filter_complex','[1:a][2:a]amerge=inputs=2,pan=5.1|FL=c1|FR=c1|FC=c0|LFE=c1|BL=c1|BR=c1[a]','-map','0:v','-map','[a]'] + flags
        ff(*(video if vc else []),'-i',tmp/'track.wav',*flags,'-c:a',ac,'-shortest',fixtures/(stem+'.'+ext))
        truth(stem,lang,lines)
        if lang=='en':
            ff(*video,'-i',tmp/'track.wav','-c:v','h264_videotoolbox','-b:v','300k','-c:a','aac','-shortest',tmp/'loop.mp4')
            # Explicit loop duration keeps audio and video aligned across repetitions.
            sec=len(pcm)/32000
            with (tmp/'loop.txt').open('w') as f:
                for i in range(int(1200/sec)+2): f.write(f"file 'loop.mp4'\nduration {sec:.8f}\n")
            ff('-f','concat','-safe',0,'-i',tmp/'loop.txt','-t',1200,'-c','copy',fixtures/'fixture_long_en.mp4')
            truth('fixture_long_en','en',[dict(row,onsetMs=round(i*sec*1000+row['onsetMs'])) for i in range(int(1200/sec)+1) for row in lines if i*sec+row['onsetMs']/1000+2<1200])
        print(stem,flush=True)
    ff(*video,'-f','lavfi','-i','sine=frequency=440:duration=12','-f','lavfi','-i','anoisesrc=duration=12:amplitude=0.01','-filter_complex','[1:a][2:a]amix[a]','-map','0:v','-map','[a]','-t',12,'-c:v','h264_videotoolbox','-b:v','500k','-c:a','aac',fixtures/'fixture_music.mp4')
    truth('fixture_music',None,[])
demo=Path('app/ui/public/demo/demo.mp4')
demo.parent.mkdir(parents=True,exist_ok=True)
shutil.copyfile(fixtures/'fixture_ja.mp4',demo)
