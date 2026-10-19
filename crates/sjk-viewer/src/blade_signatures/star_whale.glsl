// Star Whale: the song of the purrgil, the whales that swim through hyperspace, held as a blade. No
// whales are drawn: the blade is their song. A deep-blue bioluminescent blade swells and breathes as
// slow luminous waves roll up it; with each note a soft ring of song spreads out from it in every
// direction, slowly, long after it was sung. Luminous plankton and blue nebula gas drift and swirl
// round it in the dark. Every so often a hyperspace shimmer rushes up the blade with streaks of
// starlight, as a purrgil jumps. A swing quickens the waves; a clash sings out a bright ring.
// @room 8
// @palette {"corona.rim_cool":[0.04,0.10,0.45],"corona.rim_hot":[0.20,0.45,1.0],"corona.inner":[0.65,0.82,1.0],"core.white":[0.90,0.95,1.0],"core.fringe_cool":[0.15,0.35,1.0],"core.fringe_hot":[0.60,0.80,1.0],"light.color":[0.22,0.45,1.0],"trail":[0.28,0.50,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// the jump: 0..1 while a hyperspace shimmer runs up the blade (every 11 s), and its front
float swh_jump(){ float p = mod(uTime - 3.0, 11.0); return smoothstep(0.0, 0.15, p)*(1.0 - smoothstep(1.1, 1.6, p)); }
float swh_front(){ return mod(uTime - 3.0, 11.0)/1.2; }

// one ring of song: height h, radius R, thickness w; integrated soft torus density along the ray
float swh_ring(vec3 ro, vec3 rd, float h, float R, float w){
  vec2 sp = raySpan(ro, rd, R + 2.5*w, h - 2.5*w, h + 2.5*w);
  if (sp.y <= sp.x) return 0.0;
  float dt = (sp.y - sp.x)/6.0;
  float t = sp.x + dt*0.5;
  float acc = 0.0;
  for (int i = 0; i < 6; i++){
    vec3 p = ro + rd*t;
    float r = length(p.xz);
    float a = atan(p.x, p.z);
    float yy = p.y - h - 0.25*w*sin(a*3.0 + uTime*0.7);
    float d2 = (r - R)*(r - R) + yy*yy*0.55;
    acc += exp(-d2/(w*w))*dt*(0.65 + 0.35*sin(a*2.0 - uTime*0.5 + h));
    t += dt;
  }
  return acc;
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 deep = vec3(0.10, 0.28, 1.0);
  vec3 bio = vec3(0.35, 0.70, 1.0);
  vec3 pale = vec3(0.80, 0.92, 1.0);
  float jump = swh_jump()*uFx;
  float jf = swh_front()*L;

  // the blade: it breathes, swelling where the slow waves pass
  float s;
  float dA = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float ya = mix(-0.6, top, s);
  float tA; rayPt(ro, rd, vec3(0.0, ya, 0.0), tA);
  float wph = ya*0.32 - uTime*(1.1 + 1.6*sw);
  float wave = 0.5 + 0.5*sin(wph);
  wave = wave*wave;
  float wave2 = 0.5 + 0.5*sin(ya*0.11 - uTime*0.45 + 1.0);
  float shim = jump*exp(-pow((ya - jf)/3.0, 2.0));
  float gw = 0.40 + 0.45*wave + 0.15*wave2;
  vec3 light = vec3(1.0)*exp(-pow(dA/max(0.20 + 0.06*wave, px), 2.0))*(1.1 + 0.4*wave + 1.2*shim)
    + mix(deep, bio, 0.35*wave + 0.4*shim)*halo(dA, gw)*(0.6 + 1.1*wave + shim)
    + deep*halo(dA, gw*3.5)*(0.20 + 0.18*wave2 + 0.2*sw)
    + deep*halo(dA, 7.0)*0.05;

  // rings of song: three slow notes on long, staggered cycles, plus the clash
  float songs = 0.0;
  float songC = 0.0;
  if (fxOn > 0.001){
    for (int k = 0; k < 3; k++){
      float fk = float(k);
      float per = 7.0 + 1.3*fk;
      float f = fract((uTime + fk*2.9)/per);
      float h = L*(0.30 + 0.25*fk) + f*2.5;
      if (h < top){
        float R = 0.6 + 6.2*(1.0 - pow(1.0 - f, 2.2));
        float w = 0.55 + 1.3*f;
        float g = smoothstep(0.0, 0.06, f)*(1.0 - smoothstep(0.45, 1.0, f));
        songs += swh_ring(ro, rd, h, R, w)*g*(1.0 + 0.6*sw)/(0.5 + 0.6*f);
      }
    }
    // the clash sings one fast bright ring from mid blade
    if (cl > 0.01){
      float f = 1.0 - cl;
      songC = swh_ring(ro, rd, L*0.55, 0.8 + 5.5*f, 0.6 + 0.9*f)*cl;
    }
  }
  light += (mix(bio, pale, 0.15)*songs*0.32 + pale*songC*1.0)*fxOn*(0.35 + 0.65*smoothstep(0.05, 0.5, abs(rd.y)));

  // nebula gas and plankton glow drifting and swirling round the blade
  vec3 ms = marchSteps(ro, rd, 6.2, -0.5, L + 2.0, 20.0);
  if (ms.y > ms.x && fxOn > 0.001){
    float t = ms.x + ms.z*h21(rd.xy*517.0 + rd.z*37.0);
    vec3 acc = vec3(0.0);
    for (int i = 0; i < 20; i++){
      vec3 p = ro + rd*t;
      if (p.y < top + 1.0){
        float r = length(p.xz);
        float a = uTime*0.10 + p.y*0.04 + 0.6/(0.6 + r);
        vec2 q = rot(a)*p.xz;
        float yy = p.y*0.12 - uTime*0.05;
        float n = fbm3(vec2(q.x*0.30, yy + q.y*0.12))*fbm3(vec2(q.y*0.30 + 5.0, yy*1.3 - q.x*0.1));
        float sh = smoothstep(0.6, 1.6, r)*(1.0 - smoothstep(3.4, 5.8, r))*smoothstep(-0.5, 3.0, p.y)*(1.0 - smoothstep(L - 2.0, L + 2.0, p.y));
        float d = smoothstep(0.12, 0.45, n*1.4)*sh;
        // light from the blade: brighter where a wave passes at this height
        float wv = 0.5 + 0.5*sin(p.y*0.32 - uTime*(1.1 + 1.6*sw));
        float jl = jump*exp(-pow((p.y - jf)/4.0, 2.0));
        acc += mix(deep, bio, 0.3 + 0.4*wv*wv)*d*(0.6 + 0.6*wv*wv + 1.5*jl)*exp(-r*0.25);
      }
      t += ms.z;
    }
    light += acc*ms.z*0.22*fxOn;
  }

  // plankton: tiny motes of living light drifting slowly round it
  for (int i = 0; i < 26; i++){
    float fi = float(i);
    float yb = L*(0.04 + 0.96*h11(fi*3.1));
    float rr = 1.2 + 3.6*h11(fi*5.7);
    float a = h11(fi*1.3) + uTime*(0.012 + 0.02*h11(fi*9.1))/(0.4 + 0.15*rr);
    vec3 q = around(rr, a, yb + 0.8*sin(uTime*0.3 + fi));
    if (q.y < top){
      float tt; float d = rayPt(ro, rd, q, tt);
      float wv = 0.5 + 0.5*sin(q.y*0.32 - uTime*(1.1 + 1.6*sw));
      float br = (0.35 + 0.65*pow(0.5 + 0.5*sin(uTime*(0.9 + h11(fi*2.3)) + fi*2.0), 3.0) + 0.6*wv*wv)*(1.0 + 3.0*jump*exp(-pow((q.y - jf)/4.0, 2.0)));
      float w = max(px*1.0, 0.045);
      light += (pale*halo(d, w)*1.2 + bio*halo(d, w*4.0)*0.30)*br*fxOn;
    }
  }

  // the jump: streaks of starlight racing up the blade with the shimmer front
  if (jump > 0.001){
    for (int i = 0; i < 14; i++){
      float fi = float(i);
      float rr = 0.7 + 2.6*h11(fi*4.3);
      float a = h11(fi*7.7);
      float y1 = jf - 1.0 - 6.0*h11(fi*2.9);
      float len = 3.0 + 6.0*h11(fi*6.1);
      float y0 = max(y1 - len, 0.0);
      y1 = min(y1, top);
      if (y1 > y0){
        float sg;
        float d = raySeg(ro, rd, around(rr, a, y0), around(rr, a, y1), sg);
        float w = max(px*0.9, 0.04);
        float fade = sg*sg;
        light += (pale*halo(d, w)*1.5 + bio*halo(d, w*4.0)*0.4)*fade*jump;
      }
    }
  }
  return light;
}
