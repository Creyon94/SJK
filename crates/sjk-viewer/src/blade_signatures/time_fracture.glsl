// Time Fracture: a blade caught between worlds. Echoes of the blade stand beside it: an amber one a
// moment in the past trailing behind and a cyan one a moment in the future leading ahead. A portal
// floats beside the blade, a ring of light lined with ancient glyphs opening onto a starfield, and
// star-paths branch out from the blade like the roads of a world between worlds, light running
// along them. Shards of other moments drift away. A swing spreads the echoes wide; a clash freezes
// time: cracks shoot out and the portal flares.
// @room 8
// @palette {"corona.rim_cool":[0.30,0.32,0.50],"corona.rim_hot":[0.78,0.80,0.98],"corona.inner":[0.95,0.96,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.55,0.75,1.0],"core.fringe_hot":[1.0,0.80,0.55],"light.color":[0.85,0.88,1.0],"trail":[0.82,0.86,1.0]}

vec3 time_fracture_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 amber = vec3(1.0, 0.7, 0.3), cyan = vec3(0.4, 0.9, 1.0), silver = vec3(0.92, 0.95, 1.0);
  float on = smoothstep(-0.5, 0.3, b.y);
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  vec3 light = vec3(1.0)*exp(-pow(d/0.3, 2.0))*1.5*on + silver*halo(d, 0.8)*0.5*on + silver*halo(d, 2.0)*0.06*on;
  // the echoes: past (amber, behind) and future (cyan, ahead), leaning away from the hilt
  for (int e = 0; e < 2; e++){
    float s = e == 0 ? -1.0 : 1.0;
    float spread = (1.3 + 0.4*sin(uTime*0.9 + s) + 2.2*sw)*(1.0 - cl*0.8);
    float lean = s*spread*clamp(b.y/L, 0.0, 1.0);
    float ex = b.x - lean - s*0.2*smoothstep(0.0, 2.0, b.y);
    float de = length(vec2(ex, b.y - yc));
    float flick = e == 0 ? 0.75 + 0.25*step(0.15, h11(floor(uTime*14.0))) : 0.8 + 0.2*sin(b.y*12.0 - uTime*20.0);
    vec3 ec = e == 0 ? amber : cyan;
    light += ec*(exp(-pow(de/0.22, 2.0))*0.75 + halo(de, 0.6)*0.25)*flick*on*smoothstep(0.5, 3.0, b.y);
  }
  // the portal floating beside the blade: a glyph-lined ring with a starfield inside
  float cy = L*0.62 + 1.2*sin(uTime*0.3);
  vec2 q = b - vec2(4.4 + 0.4*sin(uTime*0.5), cy);
  float R = 2.1 + 0.15*sin(uTime*1.3) + 0.4*cl, rr = length(q), aa = atan(q.x, q.y);
  float rim = abs(rr - R);
  float glyph = step(0.5, h11(floor(aa/0.25 + uTime*0.8)))*smoothstep(0.12, 0.0, abs(rr - R + 0.25) - 0.06);
  vec3 pc = mix(vec3(0.9, 0.85, 1.0), cyan, 0.4);
  light += pc*(halo(rim, 0.06 + px)*1.6 + halo(rim, 0.4)*0.25 + glyph*0.7);
  float inside = smoothstep(R - 0.1, R - 0.3, rr);
  vec2 sp = q*3.0 + vec2(uTime*0.4, 0.0);
  vec2 sc = floor(sp);
  float star = step(0.85, h21(sc))*halo(length(fract(sp) - h22(sc)), 0.05);
  light += (vec3(0.15, 0.1, 0.3)*0.6 + vec3(1.0)*star*1.5)*inside;
  // star-paths branching off the blade, light pulses running out along them
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.12 + 0.13*fi)*L;
    float ang = 0.35 + 0.5*h11(fi*3.3);
    vec2 a0 = vec2(s*0.4, y0), a1 = a0 + vec2(s*cos(ang), sin(ang))*(4.5 + 2.0*h11(fi));
    vec2 a2 = a1 + vec2(s*cos(ang + 0.6), sin(ang + 0.6))*2.0;
    float dp = min(sdSeg(b, a0, a1), sdSeg(b, a1, a2));
    float t = fract(uTime*0.5 + fi*0.17);
    vec2 pulse = mix(a0, a1, t);
    light += vec3(0.85, 0.85, 1.0)*(halo(dp, 0.025 + px)*0.6 + halo(length(b - pulse), 0.12)*1.6);
    light += vec3(1.0)*halo(length(b - a1), 0.09)*1.2;
  }
  // clash: time freezes, cracks shoot out from the strike
  if (cl > 0.01){
    vec2 cq = b - vec2(0.0, L*0.6);
    float ca = atan(cq.y, cq.x), cr = length(cq);
    float crack = halo(abs(fract(ca*7.0/6.2832 + 0.15*vn(vec2(cr*1.5, 0.0))) - 0.5), 0.03)*step(cr, 1.0 + 8.0*(1.0 - cl));
    light += mix(amber, cyan, step(0.0, cq.x))*crack*cl*1.3 + vec3(1.0)*halo(abs(cr - 1.0 - 7.0*(1.0 - cl)), 0.1)*cl;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// ==== show (added) ====
// Time Fracture show: (gears removed) glass shards of other moments breaking off, and
// on the other), glass shards breaking off and drifting with time-echo copies trailing them,
// particles running backward and forward in time; a swing scatters the shards, a clash freezes time
// with a crack pattern shooting out and a ring.
float tfGear(vec2 q, float rad, float teeth, float rot){
  float r = length(q), a = atan(q.y, q.x) + rot;
  float tooth = smoothstep(-0.2, 0.2, sin(a*teeth))*0.18*rad;
  float outer = abs(r - rad - tooth) - 0.035;
  float inner = abs(r - rad*0.45) - 0.03;
  float spokes = 1e3;
  for (int k = 0; k < 4; k++){
    float sa = float(k)*0.785 - rot;
    vec2 d = vec2(cos(sa), sin(sa));
    spokes = min(spokes, max(abs(q.x*d.y - q.y*d.x) - 0.03, r - rad));
  }
  return min(min(outer, inner), max(spokes, rad*0.45 - r));
}
vec3 time_fracture_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 amber = vec3(1.0, 0.7, 0.3), cyan = vec3(0.4, 0.9, 1.0);
  vec3 light = vec3(0.0);
  // shards breaking off, each trailed by two fading echoes of itself
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float ph = uTime*(0.3 + 0.15*h11(fi*2.7)) + h11(fi*7.3);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.9 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 v = vec2(s*(3.0 + 3.0*h11(cy + fi)), 1.5*(h11(cy*2.0 + fi) - 0.4));
    vec2 o = vec2(s*0.4, (0.1 + 0.8*h11(cy*1.3 + fi*5.0))*L);
    vec3 sc = s < 0.0 ? amber : cyan;
    for (int e = 0; e < 3; e++){
      float fe = f - float(e)*0.07;
      if (fe < 0.0) continue;
      vec2 p = o + v*fe;
      vec2 q = b - p;
      float an = uTime*2.0 + fi - float(e)*0.3;
      q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
      float sh = max(abs(q.x)*1.2 + q.y*0.4, abs(q.y)*0.9) - 0.35;
      float k = (1.0 - f)*smoothstep(0.0, 0.08, f)*(e == 0 ? 1.0 : 0.35/float(e));
      light += sc*(clamp(-sh*8.0, 0.0, 1.0)*0.5 + halo(abs(sh), 0.03 + px)*1.2)*k;
    }
  }
  // time particles: amber ones fall back down, cyan ones race up, each with a ghost trail
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
    float x = s*(1.0 + 1.6*h11(fi*4.7));
    float y = mod(uTime*(s > 0.0 ? 7.0 : -5.0) + h11(fi*9.1)*L, L);
    for (int e = 0; e < 4; e++){
      float ye = y - float(e)*0.5*(s > 0.0 ? 1.0 : -1.0);
      light += (s < 0.0 ? amber : cyan)*halo(length(b - vec2(x, ye)), 0.07)*(1.4 - float(e)*0.32);
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return time_fracture_glow0(g, b, r, px) + time_fracture_show(b, min(px, 0.06));
}
