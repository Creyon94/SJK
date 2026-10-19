// Quantum: Creyon's Blade Lab design, reworked in full. A blade in superposition: ghost copies of it
// stand at other angles, quantum foam (particle pairs) pops in and out round it, electrons tunnel
// round two tilted orbits. A swing collapses the copies into one; a clash is a wave-function
// collapse: the copies snap in with a bright flash and a burst of particles.
// @full 1
// @room 7
// drawn in game: LAB_THIN 1.0
float o_sstep(float a,float b,float x){float t=clamp((x-a)/(b-a),0.,1.);return t*t*(3.-2.*t);}
float o_hash(float n){return fract(sin(n)*43758.5453);}
float o_hash2(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float o_noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(o_hash2(i),o_hash2(i+vec2(1.,0.)),f.x),mix(o_hash2(i+vec2(0.,1.)),o_hash2(i+vec2(1.)),f.x),f.y);}
float o_fbm(vec2 p){float s=0.,a=.5;for(int i=0;i<4;i++){s+=a*o_noise(p);p*=2.03;a*=.5;}return s;}
vec3 o_hsv(float h,float s,float v){vec3 k=clamp(abs(mod(h*6.+vec3(0.,4.,2.),6.)-3.)-1.,0.,1.);return v*mix(vec3(1.),k,s);}
float o_seg(float x,float y,float L){return length(vec2(max(0.,max(-x,x-L)),y));}
vec3 o_saber(vec3 g,vec3 c,float d,float cr,float k){float core=o_sstep(cr,cr*.35,d);float glow=exp(-d*k)*.55+exp(-d*k*.3)*.16;return c*core+g*glow*(1.-core*.6);}


vec3 q_blade(float x,float y,float L,float t,float k){
  float jit=(o_hash(floor(t*12.)+k)-.5)*.004*(1.-uSwing);
  float yy=y-jit; float dd=o_seg(x,yy,L);
  float h=.5+.5*sin(x*14.-t*3.);
  vec3 g=mix(vec3(.25,.85,1.),vec3(.7,.35,1.),h);
  float wave=.75+.25*cos(x*38.-sin(t*.7)*3.)*cos(t*2.3);
  vec3 c=o_saber(g,vec3(.92,.97,1.),dd,.015,22.)*wave;
  c*=.82+.18*cos(yy*480.-t*6.+x*20.);
  float blips=0.;
  for(int i=0;i<4;i++){float fi=float(i); float cy=floor(t*3.+o_hash(fi*3.7));
    vec2 at=vec2(o_hash(cy*1.31+fi*7.1)*L,(o_hash(cy*2.7+fi)-.5)*.02);
    blips+=exp(-length(vec2(x,y)-at)/.004)*step(.4,o_hash(cy+fi*11.));}
  return c+g*blips*1.2;
}
vec3 o_fx(float x,float y,float L,float d,float t){
  return q_blade(x,y,L,t,0.0);
}
// the quantum show round the blade (SJK units)
vec3 qm_show(vec2 b, float px){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 cA = vec3(0.25, 0.85, 1.0), cB = vec3(0.7, 0.35, 1.0);
  vec3 light = vec3(0.0);
  // quantum foam: particle pairs appear together, fly apart, swing back and annihilate in a flash
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float ph = t*(0.55 + 0.35*h11(fi*2.3)) + h11(fi*6.1);
    float cy = floor(ph), f = fract(ph);
    vec2 c = vec2((h11(cy*3.1 + fi) - 0.5)*7.0, (0.06 + 0.9*h11(cy*1.9 + fi*4.0))*L);
    c.x += sign(c.x)*1.0;
    c *= vec2(1.0 + 0.8*sw, 1.0);
    float an = 6.2832*h11(cy*5.3 + fi);
    vec2 dir = vec2(cos(an), sin(an));
    float sep = sin(f*3.14159)*(0.9 + 0.8*h11(fi*9.0));
    vec2 p1 = c + dir*sep, p2 = c - dir*sep;
    float life = smoothstep(0.0, 0.08, f)*(1.0 - smoothstep(0.85, 1.0, f));
    light += cA*halo(length(b - p1), 0.09)*life*1.6 + cB*halo(length(b - p2), 0.09)*life*1.6;
    light += mix(cA, cB, 0.5)*halo(sdSeg(b, p1, p2), 0.025)*life*0.35;
    float ann = exp(-pow((f - 0.94)/0.035, 2.0));                 // the annihilation flash
    light += vec3(0.9, 0.95, 1.0)*(halo(length(b - c), 0.18)*1.8 + halo(length(b - c), 0.8)*0.3)*ann;
  }
  // two tilted orbits round the blade, electrons tunnelling (jumping) between places on them
  for (int k = 0; k < 2; k++){
    float fk = float(k);
    float oy = L*(0.35 + 0.35*fk) + 3.0*sin(t*0.4 + fk*2.0);
    float tilt = 0.5 + 0.3*sin(t*0.3 + fk);
    vec2 q = b - vec2(0.0, oy);
    q = vec2(q.x, q.y*cos(tilt) - q.x*sin(tilt)*0.6);
    float ra = 3.6 + 0.6*fk, rb = 1.0;
    float e = length(q/vec2(ra, rb));
    float dd = abs(e - 1.0)*rb;
    light += mix(cA, cB, fk)*(exp(-pow(dd/0.035, 2.0))*0.35 + halo(dd, 0.15)*0.08)*(1.0 - 0.6*sw);
    for (int j = 0; j < 2; j++){
      float jump = floor(t*2.2 + float(j)*0.5 + fk*0.3);                 // tunnelling: no path between
      float a = 6.2832*h11(jump*1.7 + float(j)*3.0 + fk) + t*1.5;
      vec2 ep = vec2(ra*cos(a), rb*sin(a));
      vec2 w = vec2(ep.x, (ep.y + ep.x*sin(tilt)*0.6)/cos(tilt));
      float pop = 1.0 - fract(t*2.2 + float(j)*0.5 + fk*0.3);
      light += vec3(0.95, 0.98, 1.0)*halo(length(b - vec2(0.0, oy) - w), 0.11)*(1.2 + 1.5*pow(pop, 6.0));
    }
  }
  // clash: the collapse, a flash and a burst of probability out of the strike point
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    light += vec3(0.9, 0.95, 1.0)*halo(length(b - o), 1.5)*pow(cl, 3.0)*1.6;
    for (int k = 0; k < 14; k++){
      float fk = float(k);
      float an = fk*0.449 + 0.3*h11(fk);
      vec2 p = o + vec2(cos(an), sin(an))*(0.5 + 8.0*(1.0 - exp(-3.0*a)));
      light += mix(cA, cB, h11(fk*3.0))*halo(length(b - p), 0.12)*cl*1.8;
    }
  }
  float edge = 1.0 - smoothstep(7.0, 9.5, abs(b.x));
  light *= edge*smoothstep(-0.5, 1.0, b.y)*(1.0 - smoothstep(L + 0.5, L + 4.0, b.y));
  return (1.0 - exp(-light*1.2))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float S = 1.08/40.0; float x = b.y*S, y = b.x*S, L = uLength*S; float d = o_seg(x, y, L);
  vec3 c = o_fx(x, y, L, d, uTime);
  float R = .1; float e = 1.0 - o_sstep(R*0.45, R*0.98, max(d, abs(y)));
  vec3 light = max(c, vec3(0.0))*e;
  float held = mix(.45, 1.0, 1.0 - clamp(uSwing*2.0, 0.0, 1.0));
  for (int k = 0; k < 2; k++){
    float fk = float(k); float side = k == 0 ? 1.0 : -1.0;
    float a = (0.1 + 0.04*sin(uTime*1.3 + fk))*held*side*(0.6 + 0.4*cos(uTime*0.7 + fk*2.4));
    float cs = cos(a), sn = sin(a);
    float gx = x*cs - y*sn, gy = x*sn + y*cs;
    float gd = o_seg(gx, gy, L);
    float ge = 1.0 - o_sstep(0.025, 0.06, max(gd, abs(gy)));
    light += max(q_blade(gx, gy, L, uTime, fk + 1.0), vec3(0.0))*0.42*held*(0.85 + 0.15*sin(uTime*9.0 + fk*3.0))*ge;
  }
  light += qm_show(b, min(px, 0.06));
  return light*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
