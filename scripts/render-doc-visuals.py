"""Render exact documentation diagrams and an illustrative GIF. Requires Pillow.
Run: python3 scripts/render-doc-visuals.py
No camera, microphone, or application data is used.
"""
from pathlib import Path
import math
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/visuals'
OUT.mkdir(parents=True, exist_ok=True)
BG, PLUM, MUTED, AMBER, WHITE = '#F7F3F8', '#35112F', '#776777', '#FFBD35', '#FFFFFF'
BORDER, LILAC, GREEN = '#E4DAE6', '#E8DEF1', '#28785C'

def font(size, bold=False, mono=False):
    names = ([ '/System/Library/Fonts/Menlo.ttc', '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'] if mono else
             ['/System/Library/Fonts/Supplemental/Arial Bold.ttf' if bold else '/System/Library/Fonts/Supplemental/Arial.ttf',
              '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf' if bold else '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'])
    for name in names:
        if Path(name).exists():
            return ImageFont.truetype(name, size)
    raise RuntimeError('Install Arial or DejaVu Sans fonts to render documentation.')

def canvas(w, h, bg=BG):
    im = Image.new('RGB', (w, h), bg)
    return im, ImageDraw.Draw(im)

def text(d, x, y, value, size=24, color=PLUM, bold=False, mono=False):
    d.text((x, y), value, font=font(size, bold, mono), fill=color)

def box(d, xy, fill=WHITE, outline=None, radius=24):
    d.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline, width=2)

def pill(d, x, y, label, fill=LILAC, color=PLUM, size=18):
    width = d.textlength(label, font=font(size, True)) + 32
    box(d, (x, y, x+width, y+36), fill, radius=18)
    text(d, x+16, y+7, label, size, color, True)
    return width

def arrow(d, x1, y1, x2, y2, color=MUTED):
    d.line((x1, y1, x2, y2), fill=color, width=3)
    angle=math.atan2(y2-y1,x2-x1)
    pts=[(x2,y2)]+[(x2-12*math.cos(angle+a),y2-12*math.sin(angle+a)) for a in [-.5,.5]]
    d.polygon(pts, fill=color)

def save(im, name):
    im.save(OUT/name, optimize=True)

# Hero: typography and a compact, exact conceptual workflow, not a fake app screenshot.
im,d=canvas(1400,560,PLUM)
for r in range(280,0,-4):
    # Restrained concentric orbit pattern behind the workflow.
    d.ellipse((1120-r,270-r,1120+r,270+r),outline='#47213F',width=1)
pill(d,60,45,'OPEN SOURCE  /  LOCAL FIRST', '#56334E',WHITE,17)
text(d,60,112,'saywhen',36,WHITE,True)
text(d,56,180,'Look. Talk.',76,WHITE,True)
text(d,56,264,'Sent.',76,AMBER,True)
text(d,60,375,'Voice input for terminals',28,'#E3D4E2')
text(d,60,414,'and coding assistants.',28,'#E3D4E2')
text(d,60,501,'macOS + Windows  /  Beta',18,'#BDA9BC')
for i,(title,detail) in enumerate([('01  Look + talk','Face, lips and voice agree'),('02  Check','Read it. Cancel if needed.'),('03  Send','Paste, then Enter if allowed')]):
    y=100+i*132
    box(d,(820,y,1338,y+104),'#4A2944', '#6A4A63',22)
    text(d,846,y+18,title,26,AMBER if i==1 else WHITE,True)
    text(d,846,y+58,detail,21,'#E1D1DF')
    if i<2: arrow(d,1080,y+108,1080,y+126,'#A38A9D')
save(im,'readme-hero.png')

# Branching intent + privacy diagram.
im,d=canvas(1400,700)
text(d,55,36,'Two signals. One intention.',43,bold=True)
text(d,55,96,'Default camera-gated mode • all processing below happens on your computer',23,MUTED)
for y,title,sub,tag in [(175,'Camera','Face present + facing screen + lips moving','Frames stay in memory'),(355,'Microphone','Voice detected + a short audio pre-roll','Audio stays on-device')]:
    box(d,(55,y,625,y+140),WHITE,BORDER)
    text(d,80,y+20,title,29,bold=True)
    text(d,80,y+65,sub,21,MUTED)
    text(d,80,y+104,tag,18,GREEN)
arrow(d,625,245,730,322)
arrow(d,625,425,730,322)
box(d,(735,252,910,392),PLUM)
text(d,765,279,'BOTH',30,WHITE,True)
text(d,765,326,'agree',25,'#E3D4E2')
arrow(d,915,322,988,322)
box(d,(994,224,1345,420),WHITE,BORDER)
text(d,1020,250,'Capture speech',28,bold=True)
text(d,1020,301,'Local transcription',22,MUTED)
text(d,1020,339,'Clean up → preview',22,MUTED)
text(d,1020,377,'Cancel before delivery',19,GREEN)
box(d,(55,550,1345,650),LILAC)
text(d,80,570,'Optional paths are explicit.',23,bold=True)
text(d,80,607,'Hotkey and opt-in wake word can bypass the camera gate. Remote AI cleanup is off by default.',21,MUTED)
save(im,'intent-and-privacy.png')

# Mode selector diagram, readable without the GIF.
im,d=canvas(1400,650)
text(d,55,38,'Your words. Your send style.',43,bold=True)
text(d,55,100,'Choose how much confirmation you want before Enter.',24,MUTED)
items=[('01','Just pause','Pause, check, then send.','A cancelable countdown.'),('02','Nod to send','Wait for your confirmation.','An unconfirmed preview expires.'),('03','Say “send it”','End with the send phrase.','Otherwise, paste without Enter.'),('04','Type only','Paste your words.','Never press Enter automatically.')]
for i,(n,title,a,b) in enumerate(items):
    x=55+(i%2)*660; y=170+(i//2)*190
    box(d,(x,y,x+630,y+165),WHITE,BORDER)
    pill(d,x+22,y+22,n,AMBER)
    text(d,x+96,y+24,title,29,bold=True)
    text(d,x+26,y+79,a,24)
    text(d,x+26,y+118,b,21,MUTED)
text(d,55,589,'App rules and the risky-command guard can still withhold Enter in every mode.',23,MUTED)
save(im,'send-styles.png')

# Looping explanatory animation. Frame time is presentation pacing, not latency evidence.
frames=[]
W,H=1000,540
phases=[('Ready',18),('Listening',24),('Writing',12),('Check',24),('Sent',18)]
for phase,count in phases:
    for n in range(count):
        p=n/max(1,count-1)
        im,d=canvas(W,H)
        text(d,38,25,'saywhen',27,bold=True)
        text(d,640,32,'ILLUSTRATED WORKFLOW',16,MUTED,True)
        labels=['Look','Talk','Write','Check','Send']
        active=[a for a,_ in phases].index(phase)
        for i,label in enumerate(labels):
            x=40+i*192
            box(d,(x,81,x+177,124),PLUM if i==active else LILAC,radius=16)
            text(d,x+17,91,f'{i+1:02}  {label}',19,WHITE if i==active else MUTED,True)
        box(d,(40,150,960,441),PLUM)
        pill(d,66,173,phase,AMBER if phase in ['Listening','Check'] else '#E8DEF1')
        if phase=='Ready':
            text(d,70,235,'Look at your screen.',43,WHITE,True)
            text(d,70,302,'Face present. Facing the screen.',25,'#E3D4E2')
            text(d,70,347,'Speak when you are ready.',25,'#E3D4E2')
        elif phase=='Listening':
            text(d,70,234,'“cargo test dash dash release”',32,WHITE,True)
            for i in range(48):
                height=12+55*abs(math.sin(i*.77+n*.38))*abs(math.cos(i*.23+n*.17))
                x=75+i*17
                box(d,(x,345-height/2,x+8,345+height/2),AMBER,radius=4)
            text(d,70,399,'Lips moving + voice detected',18,'#E3D4E2')
        elif phase=='Writing':
            text(d,70,240,'Speech becomes text, locally.',37,WHITE,True)
            text(d,70,315,'Whisper → developer cleanup',25,'#E3D4E2')
            for i in range(3):
                d.ellipse((75+i*30,376,87+i*30,388),fill=AMBER if (n//3)%3==i else '#795F74')
        elif phase=='Check':
            text(d,70,237,'cargo test --release',35,WHITE,mono=True)
            text(d,70,297,'Target: trusted terminal • auto-Enter allowed',22,'#E3D4E2')
            box(d,(70,350,920,360),'#6A4A63',radius=5)
            if p<1: box(d,(70,350,70+max(10,int(850*(1-p))),360),AMBER,radius=5)
            text(d,70,390,'Esc or head shake to cancel',21,AMBER)
        else:
            text(d,70,236,'$ cargo test --release',34,WHITE,mono=True)
            text(d,70,312,'Pasted. Enter pressed.',29,AMBER,True)
            text(d,70,371,'Ready for your next thought.',24,'#E3D4E2')
        text(d,40,470,'Pause mode shown. App rules still apply.',21,MUTED)
        text(d,40,507,'Concept animation, not a screen recording. Timing is simplified.',17,MUTED)
        if phase=='Check' and n==0: save(im,'workflow-still.png')
        frames.append(im.quantize(colors=96))
frames[0].save(OUT/'workflow.gif',save_all=True,append_images=frames[1:],duration=125,loop=0,optimize=True,disposal=2)
for path in sorted(OUT.glob('*')):
    print(f'{path.relative_to(ROOT)}: {path.stat().st_size / 1024:.0f} KB')
