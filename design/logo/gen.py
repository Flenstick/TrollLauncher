S=70; A=S*0.866; B=S/2
CUBES=[(-1,2),(0,2),(1,2),(0,1),(0,0)]
def cubes(top,left,right,gap,gw):
    out=[]; xs=[]; ys=[]
    for i,k in sorted(CUBES,key=lambda q:(q[1],q[0])):
        x=i*A; y=i*B-k*S
        faces=[(top,[(x,y),(x+A,y+B),(x,y+2*B),(x-A,y+B)]),
               (left,[(x-A,y+B),(x,y+2*B),(x,y+2*B+S),(x-A,y+B+S)]),
               (right,[(x,y+2*B),(x+A,y+B),(x+A,y+B+S),(x,y+2*B+S)])]
        for col,pts in faces:
            xs+= [p[0] for p in pts]; ys+=[p[1] for p in pts]
            d='M'+' L'.join(f'{px:.2f} {py:.2f}' for px,py in pts)+' Z'
            out.append(f'<path d="{d}" fill="{col}" stroke="{gap}" stroke-width="{gw}" stroke-linejoin="round"/>')
    return ''.join(out),(min(xs),min(ys),max(xs),max(ys))
def fit(body,bb,box,size=512):
    x0,y0,x1,y1=bb; w=x1-x0; h=y1-y0; sc=box/max(w,h)
    tx=size/2-(x0+w/2)*sc; ty=size/2-(y0+h/2)*sc
    return f'<g transform="translate({tx:.2f} {ty:.2f}) scale({sc:.4f})">{body}</g>',sc
def svg(b,vb='0 0 512 512'): return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}">{b}</svg>'
# mark on dark: fill 92% of canvas
body,bb=cubes('#f2676b','#e5484d','#b4323a','#121316',5)
mark,_=fit(body,bb,470)
open('mark.svg','w').write(svg(mark))
# transparent mark (for app header)
body_t,_=cubes('#f2676b','#e5484d','#b4323a','none',0)
open('mark-flat.svg','w').write(svg(fit(body_t,bb,470)[0]))
# icon: white T on red squircle, T fills ~76%
SQ='<rect width="512" height="512" rx="116" fill="#e5484d"/>'
ib,bb=cubes('#ffffff','#ffe0e1','#ffb4b8','#e5484d',6)
icon,_=fit(ib,bb,390)
open('icon.svg','w').write(svg(SQ+icon))
# lockup
lock=f'<rect width="1200" height="240" fill="#121316"/><g transform="translate(30 20) scale(0.39)">{mark}</g><text x="236" y="150" font-family="Inter" font-weight="700" font-size="80" fill="#f4f4f6" letter-spacing="-1.5">TrollLauncher</text>'
open('lockup.svg','w').write(svg(lock,'0 0 1200 240'))
