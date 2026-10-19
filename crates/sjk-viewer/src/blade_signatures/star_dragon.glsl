// Star Dragon: the power of the Duinuogwuin, the star dragons of legend, held as a blade. No dragon
// is drawn: the blade itself is the dragon. Its body is sheathed in overlapping scales of gold light
// that wrap all the way round it and ripple upward like a living hide; a deep inner fire pulses
// slowly up its heart. Dragon breath of stardust-fire rolls up the blade and peels off it near the
// tip, full of glinting star sparks, and dark-gold smoke curls round the hilt. A swing makes the
// scales flare open and blaze and the breath roar; a clash flashes the whole hide white-hot.
// @room 10
// @palette {"corona.rim_cool":[0.45,0.26,0.04],"corona.rim_hot":[1.0,0.66,0.18],"corona.inner":[1.0,0.88,0.60],"core.white":[1.0,0.97,0.90],"core.fringe_cool":[1.0,0.60,0.15],"core.fringe_hot":[1.0,0.86,0.55],"light.color":[1.0,0.68,0.24],"trail":[1.0,0.70,0.28]}


// the scale hide at point p on the shell: light of the scale that covers p (shingled, free edges
// toward the hilt), flare = how far they stand open
float sdr_scale(vec3 p, float flare, float aa){
  float N = 6.0;
  float u = (atan(p.x, p.z)/6.28318 + 0.5 + p.y*0.012)*N;
  float v = (p.y - uTime*0.35)/0.85;
  float best = 0.0; float found = 0.0;
  float r0 = floor(v*2.0);
  for (int m = 2; m >= -1; m--){
    float rr = r0 + float(m);
    float off = 0.5*mod(rr, 2.0);
    float cu = floor(u - off + 0.5) + off;
    vec2 q = vec2(u - cu, (v - rr*0.5)*1.25);
    float d = length(q)/0.62;
    if (found < 0.5 && d < 1.0){
      found = 1.0;
      float rid = h21(vec2(mod(cu, N), rr));
      float edge = smoothstep(0.62, 0.93, d)*(1.0 - smoothstep(0.93, 1.0, d + aa*0.5));
      float body = 0.10 + 0.28*d*d + 0.12*rid;
      // a lengthwise ridge down each scale
      float ridge = halo(abs(q.x), 0.05 + aa)*smoothstep(0.1, 0.8, d)*0.35;
      best = body + edge*(1.1 + 1.6*flare) + ridge;
      // under an opened scale the inner fire shows through its lower half
      best += flare*smoothstep(0.2, 0.9, d)*step(q.y, 0.0)*0.6;
    }
  }
  return best;
}
vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 gold = vec3(1.0, 0.66, 0.20);
  vec3 ember = vec3(1.0, 0.45, 0.10);
  vec3 hot = vec3(1.0, 0.90, 0.65);
  float flare = clamp(sw*0.9 + cl, 0.0, 1.0);

  // The body and its scale hide are Sol's glow and core from the file (sig_glow); here only
  // the effects, each a piece placed round it in 3D.
  float s;
  vec3 light = vec3(0.0);
  float roar = sw*0.8 + cl*0.5;
  // dragon breath: tongues of stardust-fire rolling up the blade and peeling off near the tip
  for (int i = 0; i < 22; i++){
    float fi = float(i);
    float rate = 0.35 + 0.2*h11(fi*2.9);
    float f = fract(uTime*rate*(1.0 + roar) + h11(fi*6.1));
    float cy = floor(uTime*rate*(1.0 + roar) + h11(fi*6.1));
    float y0 = L*(0.06 + 0.7*h11(fi*3.3 + cy));
    float yy = y0 + f*(6.0 + 4.0*roar);
    if (yy > top) continue;
    float yl = clamp(yy/L, 0.0, 1.0);
    vec3 p = around(0.8 + (1.8 + 3.6*yl*yl + 1.5*roar)*f, h11(fi*7.7 + cy) + 0.2*f, yy);
    float tt;
    float d = rayPt(ro, rd, p, tt);
    if (d > 3.0) continue;
    float heat = 1.0 - f;
    float w = 0.5 + 1.0*f;
    light += mix(ember, mix(gold, hot, heat), 0.4 + 0.6*heat)*(halo(d, w)*0.7 + halo(d, w*2.0)*0.15)*sin(3.1416*f)*(0.8 + 0.4*roar)*fxOn;
  }

  // star sparks riding the breath up and off the blade
  for (int i = 0; i < 22; i++){
    float fi = float(i);
    float rate = 0.18 + 0.12*h11(fi*3.7);
    float f = fract(uTime*rate*(1.0 + roar) + h11(fi*1.9));
    float y0 = L*(0.3 + 0.5*h11(fi*5.3));
    vec3 q = around(0.7 + 3.4*f*f + roar*1.5*f, h11(fi*7.1) + f*0.45, y0 + f*(9.0 + 5.0*h11(fi)));
    if (q.y < top){
      float tt; float d = rayPt(ro, rd, q, tt);
      float tw = sin(f*3.1416)*(0.6 + 0.4*sin(uTime*13.0 + fi*4.0));
      float w = max(px*1.1, 0.05);
      light += (vec3(1.0)*halo(d, w)*2.4 + hot*halo(d, w*4.0)*0.5)*tw*fxOn;
    }
  }

  // smoke curling up from the hilt: dim warm puffs
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float f = fract(uTime*0.3 + fi/8.0);
    vec3 p = around(0.9 + 0.8*f, fi*0.37 + f*0.5 - uTime*0.1, -0.3 + 4.0*f);
    float tt;
    float d = rayPt(ro, rd, p, tt);
    light += vec3(0.55, 0.38, 0.18)*halo(d, 0.5 + 0.6*f)*0.25*sin(3.1416*f)*fxOn;
  }

  // a clash: a burst of light off the hide
  float tc;
  light += hot*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), tc), 3.0)*cl*0.5*fxOn;
  return light;
}

// the blade: Sol's glow and core from the file sheathed in the dragon's scale hide: shingled
// scales of gold light rippling up it, a slow inner fire pulsing up its heart, the scales
// flaring open and blazing on a swing, white-hot on a clash
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float flare = clamp(sw*0.9 + cl, 0.0, 1.0);
  float pulse = 0.5 + 0.5*sin(b.y*0.35 - uTime*2.2);
  pulse = pulse*pulse*pulse;
  float Rs = 0.85 + 0.2*flare;
  float sc = 0.0;
  if (abs(b.x) < Rs){
    vec3 p = vec3(b.x, b.y, sqrt(Rs*Rs - b.x*b.x));
    sc = sdr_scale(p, flare, px/0.4)*smoothstep(-0.4, 0.4, b.y)*(1.0 - smoothstep(uLength - 1.6, uLength - 0.2, b.y));
    float rip = smoothstep(0.75, 1.0, sin(b.y*0.25 - uTime*1.8));
    sc *= 0.55*(0.8 + 0.6*rip + 0.4*pulse)*(1.0 - smoothstep(Rs*0.8, Rs, abs(b.x)));
  }
  vec3 gold = vec3(1.0, 0.66, 0.20), hot = vec3(1.0, 0.90, 0.65);
  return g*(0.8 + 0.4*pulse + 0.4*flare) + mix(gold, hot, clamp(sc - 0.8, 0.0, 1.0) + cl*0.6)*sc;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*(1.0 + 0.5*clamp(uClash, 0.0, 1.0)); }

// ---- round 9: the big effects layer ----
// Creyon's round 9: every mythical dramatic, with big effects all round it in 3D (lab.glsl fx*).
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);

  vec3 gold = vec3(1.0, 0.68, 0.24), white = vec3(1.0, 0.92, 0.75);
  fx += fxCoils(ro, rd, px, y0, y1, 2.2, 0.75, 0.55 + 0.6*sw, 2.0, gold);
  fx += fxSparks(ro, rd, px, y0, y1, 22.0 + 10.0*sw, 6.5, 0.8, 7.1, white);
  fx += fxRays(ro, rd, px, y0, y1, 12.0, 7.0 + 2.0*sw, 0.55, 3.8, gold);
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float cyc = t*0.4 + fi/3.0, k = floor(cyc), q = fract(cyc);
    vec3 p = around(2.0 + 3.0*h11(k + fi), h11(k*2.3 + fi), mix(y0, y1, h11(k*4.1 + fi*1.7)));
    fx += fxFlare(ro, rd, p, px, 0.15, 2.5, 4.0, t*0.4 + fi, gold)*sin(3.1416*q);
  }
  fx += fxMotes(ro, rd, px, y0, y1, 1.2, 5.5, 26.0, 0.09, 5.2, gold);
  fx += fxAura(ro, rd, y0, y1, 3.0, gold);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 10.0, white)*1.8;
  return col + fx*uFx;
}
