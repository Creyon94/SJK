// Nebula Song: a deep-space nebula that sings, held as a blade. A deep-blue luminous blade swells
// and breathes as slow waves of light roll up it; with each note a ring of song spreads out from it
// in every direction. Three ribbons of glowing nebula gas wind slowly round it, soft blue clouds and
// living motes of light drift and swirl in the dark, and every so often a hyperspace shimmer rushes
// up the blade with streaks of starlight. A swing quickens the waves and the ribbons; a clash sings
// out a bright ring.
// @part Nebula ribbons
// @part Rings of song
// @wisps Nebula clouds
// @motes Starlight
// @room 10
// @palette {"corona.rim_cool":[0.04,0.10,0.45],"corona.rim_hot":[0.20,0.45,1.0],"corona.inner":[0.65,0.82,1.0],"core.white":[0.90,0.95,1.0],"core.fringe_cool":[0.15,0.35,1.0],"core.fringe_hot":[0.60,0.80,1.0],"light.color":[0.22,0.45,1.0],"trail":[0.28,0.50,1.0]}


// the jump: 0..1 while a hyperspace shimmer runs up the blade (every 11 s), and its front
float ns_jump(){ float p = mod(uTime - 3.0, 11.0); return smoothstep(0.0, 0.15, p)*(1.0 - smoothstep(1.1, 1.6, p)); }
float ns_front(){ return mod(uTime - 3.0, 11.0)/1.2; }

// one ring of song: height h, radius R, softness w; the ray's distance to the ring gives a
// soft band of light that keeps its shape from any side
float ns_ring(vec3 ro, vec3 rd, float h, float R, float w){
  float best = 1e3;
  vec3 q = rayAt(ro, rd, h);
  if (q.z > 0.0) best = abs(length(q.xy) - R);
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float dsc = b*b - a*c;
  if (dsc > 0.0 && a > 1e-6){
    float k = sqrt(dsc);
    float t0 = (-b - k)/a, t1 = (-b + k)/a;
    if (t0 > 0.0) best = min(best, abs(ro.y + rd.y*t0 - h));
    if (t1 > 0.0) best = min(best, abs(ro.y + rd.y*t1 - h));
  }
  return halo(best, max(0.05, w*0.15))*1.4 + halo(best, w*0.45)*0.35;
}

vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 deep = vec3(0.10, 0.28, 1.0);
  vec3 bio = vec3(0.35, 0.70, 1.0);
  vec3 pale = vec3(0.80, 0.92, 1.0);
  float jump = ns_jump()*uFx;
  float jf = ns_front()*L;

  // The blade is Sol's glow and core from the file, breathing with the slow waves
  // (sig_glow, sig_core); here only the effects, each a piece placed round it in 3D.
  vec3 light = vec3(0.0);

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
        float R = 0.6 + 7.0*(1.0 - pow(1.0 - f, 2.2));
        float w = 0.55 + 1.3*f;
        float g = smoothstep(0.0, 0.06, f)*(1.0 - smoothstep(0.45, 1.0, f));
        songs += ns_ring(ro, rd, h, R, w)*g*(1.0 + 0.6*sw)/(0.5 + 0.6*f);
      }
    }
    // the clash sings one fast bright ring from mid blade
    if (cl > 0.01){
      float f = 1.0 - cl;
      songC = ns_ring(ro, rd, L*0.55, 0.8 + 5.5*f, 0.6 + 0.9*f)*cl;
    }
  }
  light += (mix(bio, pale, 0.15)*songs*0.6 + pale*songC*1.0)*fxOn;

  // nebula gas drifting and swirling round the blade: soft deep-blue puffs, lit where a
  // wave passes and when it jumps
  for (int i = 0; i < 14; i++){
    float fi = float(i);
    float r = 1.4 + 3.0*h11(fi*4.7);
    float yy = L*(0.06 + 0.88*h11(fi*2.3));
    if (yy > top + 1.0) continue;
    vec3 p = around(r, h11(fi*8.1) + uTime*0.1/(0.6 + 0.3*r), yy + 1.0*sin(uTime*0.2 + fi));
    float tt;
    float d = rayPt(ro, rd, p, tt);
    if (d > 4.0) continue;
    float wv = 0.5 + 0.5*sin(yy*0.32 - uTime*(1.1));
    float jl = jump*exp(-pow((yy - jf)/4.0, 2.0));
    light += mix(deep, bio, 0.3 + 0.4*wv*wv)*(halo(d, 0.9 + 0.6*h11(fi))*0.3 + halo(d, 2.0)*0.06)*(0.6 + 0.6*wv*wv + 1.5*jl)*fxOn;
  }

  // ribbons of glowing nebula gas winding slowly round the blade
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    vec3 prev = around(1.6, fk/3.0 - uTime*(0.05), 0.5);
    for (int j = 1; j <= 12; j++){
      float u = float(j)/12.0;
      float yy = 0.5 + u*(L - 1.0);
      vec3 q = around(1.6 + 1.2*sin(u*6.2832 + uTime*0.4 + fk*2.0), fk/3.0 + u*0.9 - uTime*(0.05), yy);
      float sg;
      float d = raySeg(ro, rd, prev, q, sg);
      float wv = 0.5 + 0.5*sin(yy*0.32 - uTime*(1.1));
      if (yy < top) light += mix(deep, bio, 0.4 + 0.4*wv)*(halo(d, 0.25)*0.6 + halo(d, 0.8)*0.18)*(0.6 + 0.8*wv*wv)*fxOn;
      prev = q;
    }
  }
  // living motes of light: tiny motes of living light drifting slowly round it
  for (int i = 0; i < 26; i++){
    float fi = float(i);
    float yb = L*(0.04 + 0.96*h11(fi*3.1));
    float rr = 1.2 + 3.6*h11(fi*5.7);
    float a = h11(fi*1.3) + uTime*(0.012 + 0.02*h11(fi*9.1))/(0.4 + 0.15*rr);
    vec3 q = around(rr, a, yb + 0.8*sin(uTime*0.3 + fi));
    if (q.y < top){
      float tt; float d = rayPt(ro, rd, q, tt);
      float wv = 0.5 + 0.5*sin(q.y*0.32 - uTime*(1.1));
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
// the blade: Sol's glow and core from the file, breathing, swelling where the slow waves pass,
// shimmering as it jumps
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wave = 0.5 + 0.5*sin(b.y*0.32 - uTime*(1.1));
  wave = wave*wave;
  float wave2 = 0.5 + 0.5*sin(b.y*0.11 - uTime*0.45 + 1.0);
  float shim = ns_jump()*uFx*exp(-pow((b.y - ns_front()*uLength)/3.0, 2.0));
  return g*(0.6 + 0.9*wave + 0.2*wave2 + 1.2*shim + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float shim = ns_jump()*uFx*exp(-pow((b.y - ns_front()*uLength)/3.0, 2.0));
  return c*(1.0 + shim);
}

// ---- round 9: the big effects layer ----
// Round 11 (Creyon): its own look, Sol's way: its round-8 effects as part 0, its own parts
// (`part(k)`, the `// @part` lines) and Sol's pieces from its file; nothing shared with the others.
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px)*part(0.0);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 blue = vec3(0.25, 0.5, 1.0);
  for (int i = 0; i < 3; i++){
    float q = fract(t*0.25 + float(i)/3.0);
    fx += fxShock(ro, rd, px, mix(L*0.3, L*0.85, float(i)*0.5), q, 8.0, blue)*0.8*part(1.0);
  }
  fx += fxShock(ro, rd, px, L*0.6, 1.0 - cl, 10.0, vec3(0.8, 0.9, 1.0))*1.6;
  return col + fx*uFx;
}
