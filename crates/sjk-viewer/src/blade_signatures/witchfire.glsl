// Witchfire: the dark magick of a witch clan, as a blade. A thin pale-green blade with nine slim
// tendrils of green witchfire rooted at the hilt and wound close round it; every nine seconds, and
// whenever you swing, they fan open behind the blade like a cobra's hood, then wind back in. Green
// flames circle the blade, tongues of fire lick off its sides, ichor mist pours up off it and runes
// of the old magick burn in the air round it and fade. A clash bursts the fire out in a ring.
// @room 8
// @palette {"corona.rim_cool":[0.04,0.35,0.08],"corona.rim_hot":[0.35,1.0,0.35],"corona.inner":[0.8,1.0,0.75],"core.white":[0.92,1.0,0.9],"core.fringe_cool":[0.2,0.85,0.3],"core.fringe_hot":[0.7,1.0,0.6],"light.color":[0.3,1.0,0.35],"trail":[0.35,1.0,0.4]}

float wf_fan(){ float p = mod(uTime - 9.6, 9.0);
  float show = smoothstep(0.0, 0.9, p)*(1.0 - smoothstep(2.8, 4.0, p));
  return clamp(max(show, uSwing*1.5 + uClash*0.5), 0.0, 1.0); }
// one tail: rgb, cover; z its depth
vec4 wf_tail(vec2 b, float i, float fan, float px, out float z){
  float len = (19.0 + 14.0*fract(i*0.618 + 0.15))*(1.0 - 0.18*fan);
  float s = (b.y - 0.2)/len;
  z = -1.0;
  if (s < -0.02 || s > 1.03) return vec4(0.0);
  float sc = clamp(s, 0.0, 1.0);
  float t = uTime;
  // wound round the blade, close
  float a = i*0.698 + t*0.45 + 1.9*sc + 0.25*sin(t*1.4 + i*1.3 + sc*4.0);
  float Rc = 0.95*sin(min(sc*2.2, 1.0)*1.5708);
  float xc = Rc*sin(a);
  // fanned: each tail swings out to its own place in a fan behind the blade
  float spread = (i - 4.0)/4.0;
  float xf = spread*2.7*pow(sc, 0.75) + 0.2*sin(t*2.0 + i + sc*5.0)*sc;
  float x = mix(xc, xf, fan);
  z = mix(cos(a), -0.4, fan);
  // the tail's body: slim at the root, bushy past the middle, a pointed tip
  float w = 0.05 + 0.27*pow(sin(min(sc, 1.0)*2.75), 0.8)*(1.0 - smoothstep(0.86, 1.0, sc)*0.6);
  w *= 1.0 + 0.25*fan;
  float dx = b.x - x;
  float fl = fbm3(vec2(dx*3.0 + i*5.0, b.y*0.9 - t*3.2));
  float e = abs(dx) - w*(0.65 + 0.55*fl);
  float body = smoothstep(0.08, -0.05, e);
  // fur streaming toward the tip
  float fur = 0.65 + 0.35*vn(vec2(dx/max(w, 0.05)*2.5 + i*3.0, b.y*1.8 - t*5.0));
  vec3 c = mix(vec3(0.02, 0.18, 0.04), vec3(0.15, 0.85, 0.25), smoothstep(0.05, 0.55, sc));
  c = mix(c, vec3(0.55, 1.0, 0.45), smoothstep(0.55, 0.85, sc));
  c = mix(c, vec3(0.9, 1.0, 0.85), smoothstep(0.88, 0.98, sc + (fl - 0.5)*0.08));
  c *= fur*(0.85 + 0.15*sin(t*12.0 + i*2.1 + b.y));
  c += vec3(0.45, 1.0, 0.4)*halo(abs(e), 0.04)*0.5;                  // a flickering rim
  float base = smoothstep(-0.02, 0.04, s);
  float lit = mix(0.45, 1.0, smoothstep(-0.4, 0.4, z));
  return vec4(c*lit*0.85, body*base);
}
// the nine, composited back to front without adding up to white
vec4 wf_all(vec2 b, float px){
  float fan = wf_fan();
  vec3 col = vec3(0.0); float front = 0.0, cov = 0.0;
  if (abs(b.x) > 3.6) return vec4(0.0);
  for (int pass = 0; pass < 2; pass++){
    for (int k = 0; k < 9; k++){
      float z; vec4 t = wf_tail(b, float(k), fan, px, z);
      if ((pass == 0) != (z < 0.0)) continue;
      col = mix(col, t.rgb, t.a); cov = max(cov, t.a);
      if (pass == 1) front = max(front, t.a);
    }
  }
  return vec4(col, front);
}
vec3 witchfire_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  vec4 t = wf_all(b, px);
  // the blade: thin lavender-white with a soft blue fox-fire glow
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  vec3 blade = vec3(0.25, 0.9, 0.3)*(halo(bd, 0.85)*0.75 + halo(bd, 2.0)*0.25)*smoothstep(-0.3, 0.2, b.y);
  // two onibi wisps circling the upper blade, teardrops trailing down
  vec3 orbs = vec3(0.0);
  for (int i = 0; i < 2; i++){
    float fi = float(i); float a = uTime*1.2 + fi*3.14159;
    vec2 c = vec2(1.6*sin(a), L*0.70 + fi*5.0 + 1.0*sin(uTime*0.8 + fi));
    float lit = mix(0.35, 1.0, smoothstep(-0.3, 0.3, cos(a)));
    vec2 q = b - c; q.y = q.y < 0.0 ? q.y*0.4 : q.y;
    float d = length(q);
    orbs += (vec3(0.35, 1.0, 0.35)*halo(d, 0.32)*0.8 + vec3(0.85, 1.0, 0.8)*fillD(d - 0.11, px))*lit;
  }
  return blade*(1.0 - 0.6*t.a) + t.rgb + orbs + g*0.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  vec4 t = wf_all(b, px);
  return vec3(0.85, 1.0, 0.8)*(1.0 - smoothstep(0.10, 0.38, abs(b.x)))*step(0.0, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength, b.y))*(1.0 - 0.8*t.a);
}

// ==== show (added) ====
// Kitsune show: blue fox-fire flames (kitsune-bi) float round the blade, each a teardrop flame with
// a flickering tail, bobbing and circling; flame tongues lick off the blade's sides; cherry blossom
// petals swirl past. A swing throws the fox-fires out in a ring; a clash bursts nine flames outward.
vec3 witchfire_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 blue = vec3(0.2, 0.95, 0.3), pale = vec3(0.75, 1.0, 0.65), pink = vec3(0.35, 0.9, 0.5);
  vec3 light = vec3(0.0);
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    float a = uTime*(0.5 + 0.1*fi) + fi*0.9;
    float R = 3.0 + 1.2*h11(fi*3.0) + 2.5*sw + 3.0*cl;
    vec2 p = vec2(R*sin(a), (0.12 + 0.12*fi)*L + 0.8*sin(uTime*1.3 + fi));
    float front = 0.45 + 0.55*step(0.0, cos(a));
    vec2 q = b - p;
    float fl = length(vec2(q.x*1.3, max(q.y, 0.0)*0.45 + min(q.y, 0.0)*1.2)) - 0.45 - 0.1*sin(uTime*14.0 + fi + q.y*6.0);
    light += (pale*fillD(fl + 0.25, px)*1.2 + blue*fillD(fl, px)*0.7 + blue*halo(max(fl, 0.0), 0.4)*0.5)*front;
  }
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ph = uTime*(0.6 + 0.3*h11(fi*2.7)) + h11(fi*6.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.3 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.8*h11(cy*1.9 + fi))*L;
    float u = (b.x*s - 0.4)/2.2;
    if (u < 0.0 || u > 1.0) continue;
    float dy = b.y - y0 - 1.8*u*u - 0.4*sin(u*7.0 - uTime*12.0)*u;
    light += mix(pale, blue, u)*exp(-pow(dy/(0.25*(1.0 - u) + 0.03), 2.0))*(1.0 - u)*(1.0 - f)*smoothstep(0.0, 0.1, f)*1.3;
  }
  // ichor mist pouring up off the blade and drifting away
  float mist = fbm3(vec2(b.x*0.45 + 0.6*sin(b.y*0.2 + uTime*0.7), b.y*0.22 - uTime*0.9));
  light += vec3(0.15, 0.7, 0.2)*mist*mist*0.9*exp(-abs(b.x)/3.5)*smoothstep(-1.0, 3.0, b.y)*(1.0 - smoothstep(L - 2.0, L + 4.0, b.y));
  // runes of the old magick drawn in fire: rings with strokes, appearing round the blade and fading
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float ph = uTime*0.35 + fi*0.25;
    float cy = floor(ph), f = fract(ph);
    float s = mod(cy + fi, 2.0) < 0.5 ? -1.0 : 1.0;
    vec2 c = vec2(s*(4.0 + 2.0*h11(cy + fi)), (0.15 + 0.7*h11(cy*2.3 + fi))*L);
    vec2 q = rot(uTime*0.6*s)*(b - c);
    float rr = length(q), an = atan(q.y, q.x);
    float ring = halo(abs(rr - 1.3), 0.05 + px) + halo(abs(rr - 0.95), 0.03 + px)*0.6;
    vec2 t0 = vec2(0.0, 0.9), t1 = vec2(0.78, -0.45), t2 = vec2(-0.78, -0.45);
    float tri = min(min(sdSeg(q, t0, t1), sdSeg(q, t1, t2)), sdSeg(q, t2, t0));
    float stem = sdSeg(q, vec2(0.0, -0.45), vec2(0.0, 0.35 + 0.2*h11(cy + fi*3.0)));
    float strokes = halo(min(tri, stem), 0.035 + px);
    float vis = smoothstep(0.0, 0.2, f)*(1.0 - smoothstep(0.6, 1.0, f));
    light += pale*(ring + strokes*0.8)*vis*1.1;
  }
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    light += blue*halo(abs(length(b - o) - 1.0 - 6.5*(1.0 - cl)), 0.15)*cl*1.5;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return witchfire_glow0(g, b, r, px) + witchfire_show(b, min(px, 0.06));
}
