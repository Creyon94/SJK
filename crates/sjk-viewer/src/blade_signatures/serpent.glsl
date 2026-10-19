// Serpent: Creyon's Blade Lab design, reworked in full. A golden energy serpent coiled round an emerald
// blade, now slithering forward (its coils travel up toward its head at the tip). Golden scales shed off
// it and flutter away, venom drips from its fangs and hiss rings spread when its jaws open. A swing
// makes a spirit serpent strike out from the blade; a clash bursts its shed scales out in a ring.
// @full 1
// @room 8
// drawn in game: LAB_THIN 1.4
float o_sstep(float a,float b,float x){float t=clamp((x-a)/(b-a),0.,1.);return t*t*(3.-2.*t);}
float o_hash(float n){return fract(sin(n)*43758.5453);}
float o_hash2(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float o_noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(o_hash2(i),o_hash2(i+vec2(1.,0.)),f.x),mix(o_hash2(i+vec2(0.,1.)),o_hash2(i+vec2(1.)),f.x),f.y);}
float o_fbm(vec2 p){float s=0.,a=.5;for(int i=0;i<4;i++){s+=a*o_noise(p);p*=2.03;a*=.5;}return s;}
vec3 o_hsv(float h,float s,float v){vec3 k=clamp(abs(mod(h*6.+vec3(0.,4.,2.),6.)-3.)-1.,0.,1.);return v*mix(vec3(1.),k,s);}
float o_seg(float x,float y,float L){return length(vec2(max(0.,max(-x,x-L)),y));}
vec3 o_saber(vec3 g,vec3 c,float d,float cr,float k){float core=o_sstep(cr,cr*.35,d);float glow=exp(-d*k)*.55+exp(-d*k*.3)*.16;return c*core+g*glow*(1.-core*.6);}

vec3 o_fx(float x,float y,float L,float d,float t){
  vec3 bl=o_saber(vec3(.1,1.,.45),vec3(.9,1.,.92),d,.014,22.);
  float A=.034, k=26., w=2.2; float xh=L*.9+sin(t*1.1)*.01;
  float ph=k*x-t*w; float yc=A*sin(ph); float z=cos(ph);
  float slope=A*k*cos(ph); float dy=abs(y-yc)/sqrt(1.+slope*slope);
  float tail=o_sstep(.02,.25,x); float thk=.0085*mix(.35,1.,tail);
  float bodyOn=step(.02,x)*step(x,xh-.008);
  float body=o_sstep(thk,thk*.5,dy)*bodyOn;
  float scales=.72+.28*sin(x*260.-t*w*260./k)*sin((y-yc)*900.);
  float vis=mix(.28,1.,step(0.,z));
  vec3 gold=vec3(1.,.78,.25);
  vec3 bc=gold*body*scales*vis*1.25+gold*exp(-dy*120.)*.22*bodyOn*vis;
  float hph=k*xh-t*w; float hy=A*sin(hph); vec2 hp=vec2(x-xh,y-hy);
  float jaw=o_sstep(.7,1.,.5+.5*sin(t*1.7));
  vec2 hq=hp/vec2(.028,.0145); float head=o_sstep(1.,.75,length(hq-vec2(-.1,0.)));
  float mouth=step(.3,hq.x)*step(abs(hq.y),.5*jaw); head*=1.-mouth;
  float e=exp(-length(hp-vec2(.005,.0058))*900.)+exp(-length(hp-vec2(.005,-.0058))*900.);
  float tongue=exp(-abs(hp.y-sin(t*40.)*.002)*1600.)*step(.025,hp.x)*step(hp.x,.05)*step(.5,jaw);
  return bl*(1.-body*step(0.,z)*.6)+bc+gold*head*1.3+vec3(1.,.08,.04)*e*2.2+vec3(1.,.35,.4)*tongue;
}
// the serpent's show off the blade (SJK units)
vec3 sv_show(vec2 b, float px){
  float L = uLength, t = uTime, S = 1.08/40.0;
  float A = .034, k = 26., w = 2.2;
  float xh = L*S*.9 + sin(t*1.1)*.01;
  float hy = A*sin(k*xh - t*w);
  vec2 head = vec2(hy/(S*1.4), xh/S);                 // the head, in SJK units
  float jaw = o_sstep(.7, 1., .5 + .5*sin(t*1.7));
  vec3 gold = vec3(1., .78, .25), em = vec3(.15, 1., .45);
  vec3 light = vec3(0.0);
  // the spirit serpent: a huge spectral emerald snake coiled round the blade, slithering up head first
  {
    float yH = L*0.86 + 1.2*sin(t*0.5);
    float R = 3.3 + 1.2*smoothstep(0.1, 0.6, uSwing) - 1.2*uClash;
    float spin = t*1.1;
    float best = 1e3, bu = 0.0, bz = 0.0;
    vec2 pa = vec2(0.0), headP = vec2(0.0), headDir = vec2(0.0, 1.0);
    for (int j = 0; j <= 18; j++){
      float u = float(j)/18.0;
      float a = -spin - u*4.4;
      vec2 pb = vec2(R*sin(a)*(1.0 - 0.25*u), yH - u*25.0);
      if (j == 0){ headP = pb; }
      if (j == 1){ headDir = normalize(headP - pb); }
      if (j > 0){
        vec2 ba_ = pb - pa, pa_ = b - pa;
        float hh = clamp(dot(pa_, ba_)/dot(ba_, ba_), 0.0, 1.0);
        float dd = length(pa_ - ba_*hh);
        if (dd < best){ best = dd; bu = u - (1.0 - hh)/18.0; bz = cos(a); }
      }
      pa = pb;
    }
    float th = mix(0.85, 0.1, smoothstep(0.05, 1.0, bu))*smoothstep(0.0, 0.06, bu + 0.02);
    float depth = mix(0.35, 1.0, smoothstep(-0.3, 0.3, bz));
    float body = smoothstep(th, th*0.4, best);
    // diamond scales: a lattice along and across the body
    float sa_ = bu*120.0, sx_ = best/max(th, 0.05)*3.0;
    float scl = 0.55 + 0.45*smoothstep(0.15, 0.45, abs(fract(sa_ + sx_) - 0.5) + abs(fract(sa_ - sx_) - 0.5) - 0.3);
    float rim = exp(-pow((best - th*0.85)/0.07, 2.0));
    vec3 sc = mix(vec3(0.1, 0.9, 0.45), vec3(0.6, 1.0, 0.35), 0.5 + 0.5*sin(bu*30.0 - t*3.0));
    light += (sc*body*scl*1.0 + vec3(0.7, 1.0, 0.75)*rim*1.3 + em*halo(max(best - th, 0.0), 0.5)*0.25)*depth*(1.0 - smoothstep(0.85, 1.0, bu));
    // its head: a broad wedge with two burning gold eyes, jaws opening to hiss
    vec2 hq = (b - headP)/1.45;
    hq = vec2(dot(hq, vec2(headDir.y, -headDir.x)), dot(hq, headDir));
    float hw = 0.75*(1.0 - smoothstep(-0.6, 1.3, hq.y)) + 0.25;
    float hd = max(abs(hq.x) - hw, abs(hq.y - 0.3) - 1.0);
    float open = jaw*0.5;
    hd = max(hd, -(abs(hq.x) - open*(hq.y - 0.3))*step(0.3, hq.y));   // the gaping mouth
    float hdepth = mix(0.35, 1.0, smoothstep(-0.3, 0.3, cos(-spin)));
    light += (em*fillD(hd, px)*0.6 + vec3(0.7, 1.0, 0.75)*exp(-pow(hd/0.06, 2.0))*1.2)*hdepth;
    float eyes = fillD(length((vec2(abs(hq.x), hq.y) - vec2(0.38, 0.55))*vec2(1.0, 0.6)) - 0.12, px);
    light += vec3(1.0, 0.85, 0.2)*(eyes*2.0 + halo(length(vec2(abs(hq.x), hq.y) - vec2(0.38, 0.55)), 0.35)*0.6)*hdepth;
  }
  // golden scales shedding off the coils, tumbling away and fading
  for (int i = 0; i < 16; i++){
    float fi = float(i);
    float rate = 0.35 + 0.25*h11(fi*2.3);
    float ph = t*rate + h11(fi*6.7);
    float cy = floor(ph), f = fract(ph);
    float y0 = (0.08 + 0.78*h11(cy*1.9 + fi))*L;
    float x0 = A*sin(k*y0*S - t*w)/(S*1.4);
    float sd = x0 >= 0.0 ? 1.0 : -1.0;
    vec2 v = vec2(sd*(2.0 + 3.0*h11(cy + fi*4.0))*(1.0 + uSwing), 1.0 + 1.5*h11(cy*3.0 + fi));
    vec2 p = vec2(x0, y0) + v*f;
    vec2 q = b - p;
    float an = t*(4.0 + 3.0*h11(fi)) + fi;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float flip = abs(sin(t*6.0 + fi*2.0));                  // turning over as it flutters: it flashes
    float dia = (abs(q.x)/max(0.38*flip, 0.06) + abs(q.y)/0.5) - 1.0;
    float fade = (1.0 - f)*smoothstep(0.0, 0.1, f);
    light += gold*(clamp(-dia*3.0, 0.0, 1.0)*(0.6 + 0.8*flip) + halo(max(dia, 0.0)*0.25, 0.08)*0.4)*fade;
  }
  // venom dripping from the fangs while the jaws are open, and hiss rings spreading from the head
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float f = fract(t*0.9 + fi/6.0);
    vec2 p = head + vec2(0.4, 1.0) + vec2(0.8 + 1.5*f + 0.3*h11(fi), -4.5*f*f + 0.6*f);
    float on = 0.35 + 0.65*step(0.5, o_sstep(.7, 1., .5 + .5*sin((t - f/0.9)*1.7)));
    light += em*(fillD(length((b - p)*vec2(1.0, 0.75)) - 0.22*(1.0 - 0.5*f), px) + halo(length(b - p), 0.3)*0.4)*(1.0 - f)*on*1.3;
  }
  float hiss = fract(t*1.7/6.2832 + 0.25);
  float ring = abs(length(b - head - vec2(0.0, 1.0)) - 0.6 - 5.0*hiss);
  light += em*(halo(ring, 0.08 + 0.1*hiss)*1.6 + halo(ring, 0.5)*0.3)*(1.0 - hiss)*(0.3 + 0.7*jaw);
  // swing: a spirit serpent strikes out of the blade, jaws first, and fades
  float stv = smoothstep(0.45, 0.85, uSwing);
  if (stv > 0.01){
    float y0 = L*0.55;
    float reach = 7.0*stv;
    float dmin = 1e3, along = 0.0;
    for (int j = 0; j < 12; j++){
      float u0 = float(j)/12.0, u1 = float(j + 1)/12.0;
      vec2 a0 = vec2(u0*reach, y0 + 1.6*sin(u0*9.0 - t*14.0)*u0 + u0*3.0);
      vec2 a1 = vec2(u1*reach, y0 + 1.6*sin(u1*9.0 - t*14.0)*u1 + u1*3.0);
      float dd = sdSeg(b, a0, a1);
      if (dd < dmin){ dmin = dd; along = u0; }
    }
    float th = 0.12 + 0.25*along;
    vec2 tipP = vec2(reach, y0 + 1.6*sin(9.0 - t*14.0) + 3.0);
    float hd = length((b - tipP)*vec2(0.8, 1.1)) - 0.55;
    light += mix(em, gold, along)*(exp(-pow(dmin/th, 2.0))*1.3 + halo(dmin, 0.5)*0.25)*stv;
    light += gold*(fillD(hd, px)*1.2 + halo(max(hd, 0.0), 0.4)*0.5)*stv;
  }
  // clash: the shed skin bursts off in a ring of scales
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    vec2 o = vec2(0.0, L*0.6);
    light += em*halo(length(b - o), 1.3)*pow(uClash, 3.0)*1.2;
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float an = fi*0.5236 + 0.3*h11(fi);
      vec2 p = o + vec2(cos(an), sin(an))*(0.6 + 6.5*(1.0 - exp(-3.0*a)));
      vec2 q = b - p;
      float dia = (abs(q.x) + abs(q.y))/0.3 - 1.0;
      light += gold*(clamp(-dia*3.0, 0.0, 1.0) + halo(max(dia, 0.0)*0.3, 0.1)*0.5)*uClash*1.3;
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*smoothstep(-0.5, 1.0, b.y)*(1.0 - smoothstep(L + 1.5, L + 5.0, b.y));
  return (1.0 - exp(-light*1.2))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float S = 1.08/40.0; float x = b.y*S, y = b.x*S*1.4, L = uLength*S; float d = o_seg(x, y, L);
  vec3 c = o_fx(x, y, L, d, uTime);
  float R = .11; float e = 1.0 - o_sstep(R*0.45, R*0.98, max(d, abs(y)));
  return (max(c, vec3(0.0))*e + sv_show(b, min(px, 0.06)))*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
