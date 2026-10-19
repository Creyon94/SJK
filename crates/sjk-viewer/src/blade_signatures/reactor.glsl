// Reactor: Creyon's Blade Lab design, reworked in full. The in-game blade (plasma squeezed by marching
// magnetic rings, a 30 s heat cycle: calm, heating, overload, flash) plus its show off the blade:
// glowing magnetic loops arcing between the rings with plasma flowing along them, plasma venting
// from the rings as it heats, coolant sparks during overload, and a containment shockwave with
// debris at the flash. Swing: the loops stretch and vent harder; clash: a containment breach.
// @full 1
// @room 6
// drawn in game: LAB_THIN 1.35
float o_sstep(float a,float b,float x){float t=clamp((x-a)/(b-a),0.,1.);return t*t*(3.-2.*t);}
float o_hash(float n){return fract(sin(n)*43758.5453);}
float o_hash2(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float o_noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(o_hash2(i),o_hash2(i+vec2(1.,0.)),f.x),mix(o_hash2(i+vec2(0.,1.)),o_hash2(i+vec2(1.)),f.x),f.y);}
float o_fbm(vec2 p){float s=0.,a=.5;for(int i=0;i<4;i++){s+=a*o_noise(p);p*=2.03;a*=.5;}return s;}
vec3 o_hsv(float h,float s,float v){vec3 k=clamp(abs(mod(h*6.+vec3(0.,4.,2.),6.)-3.)-1.,0.,1.);return v*mix(vec3(1.),k,s);}
float o_seg(float x,float y,float L){return length(vec2(max(0.,max(-x,x-L)),y));}
vec3 o_saber(vec3 g,vec3 c,float d,float cr,float k){float core=o_sstep(cr,cr*.35,d);float glow=exp(-d*k)*.55+exp(-d*k*.3)*.16;return c*core+g*glow*(1.-core*.6);}

vec3 o_fx(float x,float y,float L,float d,float t){
  float cyc=fract(t/30.)*30.;
  float cooling=1.-o_sstep(26.,30.,cyc);
  float hx=o_sstep(16.,22.,cyc)*cooling;
  float hy=o_sstep(21.6,22.4,cyc)*(1.-o_sstep(25.6,26.2,cyc));
  float hz=exp(-max(cyc-26.,0.)*4.)*step(26.,cyc);
  float N=5.; float sp=.1;
  float strobe=hy*(.5+.5*step(.5,fract(t*2.)));
  float throb=1.+(.08+.25*hx)*sin(t*(5.));
  float ph=x/max(L,.001)*N-t*sp*N; float fr_=fract(ph);
  float r=(.008+.012*pow(sin(3.14159*fr_),2.))*(1.+.35*hy)*throb;
  vec3 calm=vec3(.3,.7,1.);
  vec3 pc=mix(mix(calm,vec3(1.,.45,.12),hx),vec3(1.,.18,.06),hy);
  float dd=length(vec2(max(0.,max(-x,x-L)),max(0.,abs(y)-r*.45)));
  vec3 plasma=o_saber(pc*(1.+strobe*.6),mix(vec3(1.),pc,.35),dd,r*.9,30.);
  float rings=0.;
  for(int k=0;k<5;k++){float xr=fract((float(k)+t*sp*N)/N)*L; float rr=.026; float ay=min(abs(y)/rr,1.);
    float arc=xr+.004*sqrt(1.-ay*ay); rings+=exp(-abs(x-arc)*700.)*(.35+.9*pow(ay,6.))*step(abs(y),rr);}
  vec3 rc=mix(mix(vec3(.6,.95,1.),vec3(1.,.6,.3),hx),vec3(1.,.3,.15),hy);
  vec3 flash=vec3(1.,.95,.9)*hz*(exp(-d*30.)*1.5+exp(-d*8.)*.4);
  vec3 core=plasma+rc*rings*step(0.,x)*step(x,L)*(1.+strobe)+flash;
  float field=0.;
  for(int i=0;i<2;i++){float fi=float(i); float p2=x*52.-t*(3.)+fi*3.14159;
    float yc=.017*sin(p2); field+=exp(-abs(y-yc)/.0012)*(.25+.75*(.5+.5*cos(p2)));}
  field*=step(.02,x)*step(x,L-.02)*(.35+.65*hx+.5*hy);
  vec3 fc=mix(vec3(.5,.85,1.),vec3(1.,.55,.25),hx);
  float arcs=0.;
  for(int k=0;k<5;k++){float x0=fract((float(k)+t*sp*N)/N)*L; float x1=x0+L/N;
    float cy=floor(t*7.+float(k)*1.7); float on=step(.55-.5*hx,o_hash(cy*3.1+float(k)));
    float side=o_hash(cy+float(k)*5.)<.5?-1.:1.; float u=clamp((x-x0)/max(x1-x0,.001),0.,1.);
    float jag=(o_noise(vec2(x*300.,cy))-.5)*.008; float yc=side*(.02+.006*sin(u*3.14159))+jag;
    arcs+=exp(-abs(y-yc)/.0009)*step(x0,x)*step(x,x1)*on;}
  arcs*=hx*step(x,L);
  float coil=exp(-length(vec2(x,y)-vec2(.03,0.))*60.)*(.6+.4*sin(t*6.));
  return core+fc*field*.55+vec3(.85,.9,1.)*arcs*.9+mix(vec3(.4,.8,1.),vec3(1.,.5,.2),hx)*coil;
}
// the reactor's show off the blade (SJK units)
vec3 rc_show(vec2 b, float px){
  float L = uLength, t = uTime;
  float cyc = fract(t/30.)*30.;
  float cooling = 1. - o_sstep(26., 30., cyc);
  float hx = o_sstep(16., 22., cyc)*cooling;
  float hy = o_sstep(21.6, 22.4, cyc)*(1. - o_sstep(25.6, 26.2, cyc));
  float hz = exp(-max(cyc - 26., 0.)*1.6)*step(26., cyc);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float heat = max(hx, cl);
  vec3 calm = vec3(.3, .7, 1.), hot = vec3(1., .45, .12), crit = vec3(1., .18, .06);
  vec3 pc = mix(mix(calm, hot, hx), crit, hy);
  float N = 5., sp = .1;
  vec3 light = vec3(0.0);
  float side = sign(b.x + 1e-5), ax = abs(b.x);
  // magnetic loops: a glowing arch between each pair of rings on both sides, plasma beads flowing along
  for (int k = 0; k < 5; k++){
    float fk = float(k);
    float y0 = fract((fk + t*sp*N)/N)*L, y1 = y0 + L/N;
    if (y1 > L || y0 < 1.5) continue;
    float mid = (y0 + y1)*0.5, hw = (y1 - y0)*0.5;
    float H = (2.2 + 1.6*hx + 2.0*sw + 2.5*cl)*(0.85 + 0.15*sin(t*3.0 + fk*1.7 + side));
    // ellipse arch: (dy/hw)^2 + (x/H)^2 = 1, outside x > 0.5
    vec2 q = vec2((b.y - mid)/hw, (ax - 0.4)/H);
    float e = length(q);
    float dd = abs(e - 1.0)*min(hw, H)*0.8 + 1e3*step(ax, 0.4);
    float pulse = 0.55 + 0.45*sin(atan(q.y, q.x)*3.0 - t*(6.0) + fk);
    light += pc*(exp(-pow(dd/0.06, 2.0))*0.9 + halo(dd, 0.35)*0.18)*pulse*(0.45 + 0.55*hx + 0.6*sw);
    // plasma beads riding the arch
    for (int j = 0; j < 2; j++){
      float a = fract(t*(0.45) + float(j)*0.5 + fk*0.23)*3.14159;
      vec2 pp = vec2(0.4 + H*sin(a), mid - hw*cos(a));
      light += mix(pc, vec3(1.0), 0.5)*halo(length(vec2(ax, b.y) - pp), 0.13)*1.4;
    }
  }
  // venting: plasma jets squirting out of the rings, curling and fading (more as it heats)
  for (int k = 0; k < 6; k++){
    float fk = float(k);
    float ph = t*(0.7) + h11(fk*3.3);
    float cy = floor(ph), f = fract(ph);
    if (h11(cy*2.7 + fk) > 0.25 + 0.6*heat + 0.4*sw) continue;
    float ry = fract((floor(h11(cy + fk*5.0)*5.0) + t*sp*N)/N)*L;
    float s = h11(cy*4.9 + fk) < 0.5 ? -1.0 : 1.0;
    float len = (2.5 + 3.5*h11(cy*1.1 + fk))*(1.0 + 0.6*sw)*smoothstep(0.0, 0.4, f);
    float xs = b.x*s - 0.4;
    float u = clamp(xs/len, 0.0, 1.0);
    float curl = (h11(cy*6.0 + fk) - 0.5)*2.5*u*u + sin(u*9.0 - t*20.0)*0.12*u;
    float dy = b.y - ry - curl;
    float th = 0.08 + 0.35*u;
    float jet = exp(-pow(dy/th, 2.0))*step(0.0, xs)*(1.0 - smoothstep(len*0.7, len, xs))*(1.0 - f);
    light += mix(vec3(1.0), pc, smoothstep(0.0, 0.4, u))*jet*(1.1 - 0.6*u);
  }
  // overload: coolant sparks spraying off the whole blade
  float spr = max(hy, cl*0.8);
  if (spr > 0.01){
    for (int k = 0; k < 10; k++){
      float fk = float(k);
      float ph = t*1.8 + h11(fk*7.0);
      float cy = floor(ph), f = fract(ph);
      float y = h11(cy*3.0 + fk)*L;
      float s = h11(cy + fk*2.0) < 0.5 ? -1.0 : 1.0;
      vec2 dir = normalize(vec2(s, (h11(cy*5.0 + fk) - 0.5)*1.2));
      vec2 o = vec2(0.0, y), p = o + dir*f*7.0;
      light += vec3(1.0, 0.9, 0.7)*halo(sdSeg(b, o + dir*max(f*7.0 - 1.2, 0.0), p), 0.05)*(1.0 - f)*spr*1.6;
    }
  }
  // the flash: a containment shockwave ring and debris; a clash does the same at the strike point
  float sh = max(hz, cl);
  if (sh > 0.01){
    float a = hz > cl ? (cyc - 26.0)/1.5 : (1.0 - cl);
    vec2 o = vec2(0.0, hz > cl ? L*0.5 : L*0.6);
    float ring = abs(length((b - o)*vec2(1.0, 0.55)) - 1.0 - 8.0*a);
    light += mix(vec3(1.0, 0.95, 0.9), pc, 0.4)*(halo(ring, 0.15)*1.6 + halo(ring, 0.7)*0.3)*sh*(1.0 - smoothstep(0.7, 1.0, a));
    for (int k = 0; k < 8; k++){
      float fk = float(k);
      float an = fk*0.785 + 0.4*h11(fk);
      vec2 p = o + vec2(cos(an), sin(an))*(1.0 + 7.0*a);
      light += vec3(1.0, 0.7, 0.4)*halo(length(b - p), 0.18)*sh*1.4;
    }
  }
  float edge = 1.0 - smoothstep(6.5, 8.8, ax);
  return light*edge*smoothstep(-0.5, 1.0, b.y)*(1.0 - smoothstep(L + 0.5, L + 4.0, b.y));
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float S = 1.08/40.0; float x = b.y*S, y = b.x*S*1.35, L = uLength*S; float d = o_seg(x, y, L);
  vec3 c = o_fx(x, y, L, d, uTime);
  float R = .1; float e = 1.0 - o_sstep(R*0.45, R*0.98, max(d, abs(y)));
  vec3 show = rc_show(b, min(px, 0.06));
  return (max(c, vec3(0.0))*e + (1.0 - exp(-show*1.2))*1.05)*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
