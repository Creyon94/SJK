// Superlaser: a battle station's main weapon as a blade. Eight tributary beams fan in from the
// emitter's sides and meet at a blazing focus just above the hilt, and from there the main beam
// fires up to the tip: a thin white-hot core in a green glow, energy surges racing up it,
// rings of force rippling out along it and sparks crackling off. On ignition the tributaries
// light first, then the beam fires from the focus. A swing flares the tributaries and sends a
// surge; a clash fires the focus in a white flash and a shockwave ring.
// @room 8
// @palette {"corona.rim_cool":[0.05,0.55,0.12],"corona.rim_hot":[0.45,1.0,0.35],"corona.inner":[0.85,1.0,0.8],"core.white":[0.95,1.0,0.94],"core.fringe_cool":[0.2,0.9,0.3],"core.fringe_hot":[0.7,1.0,0.6],"light.color":[0.3,1.0,0.35],"trail":[0.35,1.0,0.4]}

const float SL_F = 10.0;   // the focus's height
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  px = min(px, 0.06);
  float L = uLength, t = uTime, ax = abs(b.x);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 green = vec3(0.25, 1.0, 0.3), pale = vec3(0.8, 1.0, 0.75), white = vec3(1.0);
  float lit = smoothstep(0.0, 0.18, uAge);                       // the tributaries light first
  float fire = clamp((uAge - 0.12)/0.3, 0.0, 1.0);               // then the beam fires up
  float top = mix(SL_F, L, fire);
  vec2 F = vec2(0.0, SL_F);
  vec3 light = vec3(0.0);
  // the tributaries: eight beams fanning in to the focus from beside the emitter
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
    float k = floor(fi/2.0);
    vec2 a = vec2(s*(3.2 + 1.4*k), 2.0 + 1.3*k - 0.15*k*k);
    float d = sdSeg(b, a, F);
    float flick = 0.8 + 0.2*sin(t*40.0 + fi*2.1);
    float u = clamp(dot(b - a, F - a)/dot(F - a, F - a), 0.0, 1.0);
    light += (white*halo(d, max(px, 0.04))*1.3 + green*halo(d, 0.22)*0.9 + green*halo(d, 0.8)*0.15)*flick*lit*(0.5 + 0.5*u)*(1.0 + 1.2*sw);
    light += pale*halo(length(b - a), 0.35)*lit*0.7;           // its emitter
  }
  // the focus: a blazing knot with rings pulsing off it
  float dF = length(b - F);
  float beat = 0.5 + 0.5*sin(t*7.0);
  light += white*halo(dF, 0.5 + 0.25*beat)*2.2*lit + green*halo(dF, 1.8)*0.8*lit;
  for (int j = 0; j < 2; j++){
    float f = fract(t*1.2 + float(j)*0.5);
    light += green*halo(abs(dF - 0.6 - 3.5*f), 0.08 + 0.1*f)*(1.0 - f)*0.9*lit;
  }
  // a thin feed from the emitter up to the focus, so the blade leaves the hilt
  float feed = abs(b.x);
  light += (pale*halo(feed, 0.18)*0.9 + green*halo(feed, 0.6)*0.3)*smoothstep(-0.6, 0.0, b.y)*(1.0 - smoothstep(SL_F - 0.5, SL_F, b.y))*lit;
  // the main beam from the focus to the tip
  float on = smoothstep(SL_F - 0.4, SL_F + 0.4, b.y)*(1.0 - smoothstep(top - 0.3, top + 0.4, b.y));
  float jit = (vn(vec2(b.y*3.0, floor(t*30.0))) - 0.5)*0.08;
  float bd = abs(b.x - jit);
  light += (white*fillD(bd - 0.22, px)*1.3 + pale*halo(bd, 0.35)*1.0 + green*halo(bd, 1.0)*0.55 + green*halo(bd, 2.4)*0.12)*on;
  // surges racing up the beam
  for (int j = 0; j < 3; j++){
    float fj = float(j);
    float y = SL_F + fract(t*0.9 + fj/3.0 + 0.6*sw*fj)*(L - SL_F + 2.0);
    float dd = length(vec2(bd*1.4, b.y - y));
    light += white*halo(dd, 0.45)*1.2*on + green*halo(dd, 1.5)*0.35*on;
  }
  // rings of force rippling out along the beam
  for (int j = 0; j < 4; j++){
    float fj = float(j);
    float f = fract(t*0.7 + fj*0.25);
    float y = SL_F + 2.0 + f*(L - SL_F - 2.0);
    float rr = 0.8 + 4.5*f;
    float e = abs(length(vec2(b.x, (b.y - y)*3.2)) - rr);
    float ring = halo(e, 0.1)*(1.0 - f)*step(b.y, top);
    light += pale*ring*1.4 + green*halo(e, 0.5)*(1.0 - f)*0.3*step(b.y, top);
  }
  // sparks crackling off the beam
  for (int i = 0; i < 16; i++){
    float fi = float(i);
    float ph = t*(1.3 + 0.6*h11(fi*2.7)) + h11(fi*5.3);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(0.3 + 6.5*f), SL_F + (0.1 + 0.85*h11(cy*1.9 + fi))*(L - SL_F) + 1.5*f);
    light += mix(white, green, f)*halo(length(b - p), max(px, 0.06))*2.0*(1.0 - f)*step(p.y, top);
  }
  // energy arcing off the beam and motes spiralling into the focus
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float k = floor(t*7.0 + fi*0.37);
    float y0 = SL_F + 2.0 + (L - SL_F - 4.0)*h11(k*1.3 + fi);
    float sd = h11(k + fi*7.0) < 0.5 ? -1.0 : 1.0;
    vec2 end = vec2(sd*(3.5 + 3.0*h11(k*2.1 + fi)), y0 + (h11(k*3.3 + fi) - 0.3)*4.0);
    float on = step(0.35 - 0.3*sw, h11(k*4.7 + fi));
    light += boltLight(boltD(b, vec2(0.0, y0), end, k + fi, 0.28), green, px)*on*step(b.y, top + 1.0);
  }
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float f = fract(t*0.8 + fi/12.0);
    float an = fi*0.524 + f*5.0;
    vec2 p = F + vec2(cos(an)*1.3, sin(an))*(7.0*(1.0 - f) + 0.4);
    light += mix(green, white, f)*halo(length(b - p), 0.12)*1.8*f*lit;
  }
  // a clash fires the focus: a white flash and a shockwave ring
  if (cl > 0.01){
    float rr = abs(length((b - F)*vec2(1.0, 0.7)) - 0.8 - 8.0*(1.0 - cl));
    light += white*halo(dF, 2.5)*pow(cl, 3.0)*2.0 + mix(white, green, 1.0 - cl)*halo(rr, 0.15)*cl*1.5;
  }
  float tip = length(vec2(b.x, max(b.y - L, 0.0)));
  light += green*halo(tip, 0.8)*0.4*fire*step(L - 1.0, b.y);
  float fade = (1.0 - smoothstep(8.5, 10.0, ax))*(1.0 - smoothstep(L + 0.8, L + 3.5, b.y))*smoothstep(-3.0, -1.2, b.y);
  return (1.0 - exp(-light*fade*1.1))*1.06;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
