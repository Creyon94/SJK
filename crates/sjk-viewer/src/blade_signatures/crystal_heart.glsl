// Crystal Heart: a faceted ruby heart sits on the emitter and beats, lub-dub, swelling and flashing
// on every beat. Each beat sends a pulse of light racing up the veins that branch through the thin
// rose-crystal blade above it. Every ten seconds it surges: three hard beats flood every vein to the
// tip. Swinging quickens the heartbeat; a clash makes it skip and flare.
// Each beat also throws a heart-shaped shockwave out round the blade, curling ribbons of rose light
// peel off it and tiny crystal hearts float away twinkling. Swinging sheds more hearts; a clash
// cracks the heart into flying shards. The blade's tip is rounded.
// @room 7
// @palette {"corona.rim_cool":[0.45,0.04,0.18],"corona.rim_hot":[1.0,0.30,0.50],"corona.inner":[1.0,0.80,0.88],"core.white":[1.0,0.94,0.96],"core.fringe_cool":[0.90,0.20,0.45],"core.fringe_hot":[1.0,0.65,0.78],"light.color":[1.0,0.30,0.50],"trail":[1.0,0.32,0.52]}

const float CH_S = 2.2, CH_Y = 0.2;
float ch_surge(){ float p = mod(uTime - 10.0, 10.0); return smoothstep(0.0, 0.2, p)*(1.0 - smoothstep(2.4, 3.2, p)); }
float ch_rate(){ return 1.05*(1.0 + 0.8*uSwing) + 1.2*ch_surge(); }
float ch_beatAt(float ph){ ph = max(ph, 0.0); return exp(-ph*16.0) + 0.6*exp(-max(ph - 0.22, 0.0)*16.0)*step(0.22, ph); }
float ch_beat(){ float ph = fract(uTime*ch_rate()); return ch_beatAt(ph)*(1.0 + 0.6*ch_surge()) + uClash*0.8; }
float ch_dot2(vec2 v){ return dot(v, v); }
float ch_sdHeart(vec2 p){
  p.x = abs(p.x);
  if (p.y + p.x > 1.0) return sqrt(ch_dot2(p - vec2(0.25, 0.75))) - 0.35355;
  return sqrt(min(ch_dot2(p - vec2(0.0, 1.0)), ch_dot2(p - 0.5*max(p.x + p.y, 0.0))))*sign(p.x - p.y);
}
// the heart: rgb light, a cover
vec4 ch_heart(vec2 b, float px){
  float bt = ch_beat();
  float s = CH_S*(1.0 + 0.07*bt);
  vec2 p = (b - vec2(0.0, CH_Y))/s;
  float d = ch_sdHeart(p)*s;
  if (d > 1.6) return vec4(0.0);
  // facets: sectors round a point under the cleft, split by an inner girdle
  vec2 fc = vec2(0.0, 0.62);
  vec2 v = p - fc;
  float a = atan(v.x, v.y);
  float rr = length(v);
  float n = 9.0;
  float sec = floor((a + 3.14159)/(6.28318/n));
  float fa = fract((a + 3.14159)/(6.28318/n));
  float ring = step(0.30, rr);
  float edgeA = min(fa, 1.0 - fa)*(6.28318/n)*rr*s;
  float edgeR = abs(rr - 0.30)*s;
  float edges = halo(min(edgeA, edgeR), 0.03 + px);
  float shade = 0.45 + 0.55*h11(sec*3.7 + ring*11.0);
  float glint = pow(max(0.0, sin(sec*1.9 + ring*2.0 - uTime*1.3)), 12.0);
  float inside = fillD(d, px);
  vec3 ruby = mix(vec3(0.55, 0.02, 0.12), vec3(1.0, 0.25, 0.42), shade);
  ruby = mix(ruby, vec3(1.0, 0.75, 0.85), glint*0.8);
  vec3 c = ruby*inside*(0.9 + 0.9*bt);
  c += vec3(1.0, 0.85, 0.92)*edges*inside*0.9;
  c += vec3(1.0, 0.95, 0.97)*halo(rr, 0.22)*inside*(0.6 + 2.2*bt);          // the light at its heart
  c += vec3(1.0, 0.85, 0.9)*halo(abs(d), 0.03 + px)*1.3;                    // the outline
  c += vec3(1.0, 0.25, 0.45)*halo(max(d, 0.0), 0.5 + 0.4*bt)*(0.4 + 1.0*bt); // the beat's bloom
  return vec4(c, inside);
}
// the veins: five meandering vessels from the heart's cleft, each forking into capillaries
vec3 ch_veins(vec2 b, float px){
  float L = uLength;
  float y0 = CH_Y + CH_S*0.9;
  if (b.y < y0 - 0.5 || b.y > L + 0.5) return vec3(0.0);
  float rate = ch_rate();
  float h = clamp((b.y - y0)/(L - y0), 0.0, 1.0);
  float cnt = 0.0;
  float vein = 0.0;
  // pulses in flight (the last three beats), moving up at 20 units/s
  float ph = uTime*rate;
  float pl = 0.0;
  for (int j = 0; j < 3; j++){
    float beatT = (floor(ph) - float(j))/rate;
    float yp = y0 + (uTime - beatT)*20.0;
    pl += halo(abs(b.y - yp), 1.3)*(1.0 + ch_surge());
  }
  float flood = ch_surge()*smoothstep(y0 + (mod(uTime - 10.0, 10.0))*22.0 + 2.0, y0 + (mod(uTime - 10.0, 10.0))*22.0 - 2.0, b.y);
  float conv = 1.0 - smoothstep(0.7, 1.0, h);                               // they meet at the tip
  for (int k = 0; k < 5; k++){
    float fk = float(k);
    float off = (fk - 2.0)*0.28;
    float x = (off + 0.22*sin(b.y*(0.35 + 0.07*fk) + fk*1.7) + 0.08*sin(b.y*1.3 + fk*4.0))*conv*smoothstep(y0 - 0.5, y0 + 2.0, b.y);
    float w = mix(0.07, 0.035, h);
    float d = abs(b.x - x);
    vein = max(vein, fillD(d - w, px) + halo(d, 0.10)*0.3);
    // a capillary fork every few units
    float cy = (b.y - y0)/2.6 + fk*0.37;
    float ci = floor(cy);
    float fy = fract(cy);
    float side = h11(ci + fk*13.0) < 0.5 ? -1.0 : 1.0;
    vec2 a0 = vec2(x, b.y - fy*2.6);
    vec2 a1 = a0 + vec2(side*0.38, 1.1)*conv;
    float dc = sdSeg(b, a0, a1);
    vein = max(vein, fillD(dc - 0.022, px)*0.8 + halo(dc, 0.06)*0.3);
  }
  vein = min(vein, 1.4);
  vec3 base = vec3(0.75, 0.04, 0.16)*0.8;
  return vein*(base + vec3(1.0, 0.45, 0.60)*(pl*1.5 + flood*1.6));
}
// the heart's show off the blade: beat shockwaves, rose ribbons, floating crystal hearts, clash shards
vec3 ch_show(vec2 b, float px){
  float L = uLength;
  vec3 light = vec3(0.0);
  vec2 hc = vec2(0.0, CH_Y + CH_S*0.55);
  float rate = ch_rate();
  // beat shockwaves: heart outlines growing out from the heart and fading, the last two beats
  for (int j = 0; j < 2; j++){
    float ph = fract(uTime*rate) + float(j);
    float age = ph/rate;
    float sc = CH_S*(1.0 + 2.6*age*(1.0 + 0.6*uSwing));
    vec2 p = (b - vec2(0.0, CH_Y - 0.4*age*CH_S))/sc;
    float d = abs(ch_sdHeart(p))*sc;
    float k = (1.0 - smoothstep(0.0, 1.3, age))*(1.0 + ch_surge());
    light += vec3(1.0, 0.35, 0.55)*(halo(d, 0.06 + px) + halo(d, 0.4)*0.3)*k*0.9;
  }
  // ribbons of rose light peeling off the blade, curling outward and fading
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    float ph = uTime*(0.45 + 0.2*h11(fi*2.9)) + h11(fi*7.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.3 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.15 + 0.75*h11(cy*1.7 + fi*4.0))*L;
    float len = (3.5 + 3.0*h11(cy + fi))*smoothstep(0.0, 0.45, f);
    float xs = b.x*s - 0.6;
    float u = xs/max(len, 0.01);
    if (u < 0.0 || u > 1.0) continue;
    float curl = sin(u*5.0 + cy)*1.1*u + u*u*2.2*(h11(cy*5.0 + fi) - 0.3);
    float dy = b.y - y0 - curl;
    float th = 0.05 + 0.05*u;
    float fade = (1.0 - f)*smoothstep(0.0, 0.08, f)*(1.0 - smoothstep(0.85, 1.0, u));
    light += mix(vec3(1.0, 0.75, 0.85), vec3(1.0, 0.25, 0.5), u)*(exp(-pow(dy/th, 2.0))*1.4 + halo(abs(dy), 0.3)*0.2)*fade;
  }
  // tiny crystal hearts floating up and away, twinkling
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float ph = uTime*(0.3 + 0.15*h11(fi*3.1)) + h11(fi*9.3);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*2.1 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(1.0 + 4.5*f + 0.5*sin(f*9.0 + fi)), (0.12 + 0.8*h11(cy*4.3 + fi))*L + 2.5*f);
    float sz = 0.35 + 0.25*h11(fi*5.0);
    float hd = ch_sdHeart((b - p)/sz + vec2(0.0, 0.5))*sz;
    float tw = 0.6 + 0.4*sin(uTime*7.0 + fi*3.0);
    float fade = (1.0 - f)*smoothstep(0.0, 0.1, f);
    light += vec3(1.0, 0.45, 0.65)*(fillD(hd, px)*0.9*tw + halo(max(hd, 0.0), 0.2)*0.4)*fade;
  }
  // clash: the heart cracks and shards of ruby crystal fly out
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float an = fi*0.5236 + 0.4*h11(fi);
      vec2 p = hc + vec2(cos(an), sin(an))*(0.8 + 7.0*(1.0 - exp(-3.0*a)));
      vec2 q = b - p;
      float rr = uTime*7.0 + fi;
      q = vec2(q.x*cos(rr) - q.y*sin(rr), q.x*sin(rr) + q.y*cos(rr));
      float sh = max(abs(q.x)*1.7 + abs(q.y)*0.6, abs(q.y)) - 0.32;
      light += vec3(1.0, 0.3, 0.5)*(fillD(sh, px) + halo(max(sh, 0.0), 0.25)*0.5)*uClash*1.3;
    }
    light += vec3(1.0, 0.6, 0.75)*halo(length(b - hc), 1.8)*pow(uClash, 3.0);
  }
  float edge = 1.0 - smoothstep(7.5, 9.0, abs(b.x));
  return light*edge*(1.0 - smoothstep(L + 1.0, L + 4.0, b.y))*smoothstep(-3.0, -0.5, b.y);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float ax = abs(b.x);
  float on = smoothstep(CH_Y + 1.0, CH_Y + 2.5, b.y);
  // the rose crystal blade: clear glass, long facets catching light
  float facet = halo(abs(fract(b.y*0.18 + b.x*0.35) - 0.5), 0.05)*0.25 + halo(abs(fract(b.y*0.13 - b.x*0.4 + 0.3) - 0.5), 0.04)*0.18;
  float dcap = length(vec2(b.x, max(b.y - (L - 0.8), 0.0)));         // a rounded tip, never open
  float glass = fillD(dcap - 0.80, px);
  vec3 blade = (vec3(1.0, 0.40, 0.58)*0.30 + vec3(1.0, 0.80, 0.88)*facet*0.8)*glass*on;
  blade += vec3(1.0, 0.85, 0.92)*halo(abs(dcap - 0.80), 0.03 + px)*0.6*on;              // the crystal's edge
  float bd = length(vec2(b.x, max(b.y - L, 0.0)));
  vec3 outer = vec3(1.0, 0.22, 0.45)*(halo(bd, 1.2)*0.45 + halo(bd, 2.4)*0.15)*on*(0.85 + 0.35*ch_beat());
  vec4 hrt = ch_heart(b, px);
  vec3 veins = ch_veins(b, px)*glass*(1.0 - hrt.a);
  // the tip flashes when a surge reaches it
  float tipF = ch_surge()*halo(length(b - vec2(0.0, L - 0.5)), 1.2)*smoothstep(1.4, 2.0, mod(uTime - 10.0, 10.0));
  vec3 show = ch_show(b, px);
  return (outer + blade)*(1.0 - hrt.a) + veins + hrt.rgb + vec3(1.0, 0.8, 0.9)*tipF*1.5 + show;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float on = smoothstep(CH_Y + 2.2, CH_Y + 3.4, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength, b.y));
  float dc = length(vec2(b.x, max(b.y - (uLength - 0.9), 0.0)));
  return vec3(1.0, 0.9, 0.95)*(1.0 - smoothstep(0.04, 0.18, dc))*smoothstep(CH_Y + 2.2, CH_Y + 3.4, b.y)*0.45;
}
