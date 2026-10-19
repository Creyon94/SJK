// Abyssal Eye: the blade is a slit cut into an endless dark, its lips glowing crimson, faint motes
// drifting down into the black. Along its middle the slit peels open into a great almond and an eye
// inside looks out: a burning amber iris with a goat's bar pupil, rolling to watch along the blade,
// darting, blinking, narrowing when you swing; on a clash it flinches shut. The cut is a real
// shape in space, so it keeps its form seen from any side, from above and in first person. Round
// it, shadow tendrils with crimson tips reach out in every direction and curl back, crimson tears
// drip off the lips and fall, specks of light are sucked in from all round, and jagged cracks in
// reality tear outward round the blade, pulse with the abyss's heartbeat and heal. A swing lashes
// the tendrils out; a clash flares a crimson shock ring with the tendrils whipping wide.
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.30,0.0,0.04],"corona.rim_hot":[0.95,0.12,0.10],"corona.inner":[1.0,0.45,0.30],"core.white":[1.0,0.70,0.55],"core.fringe_cool":[0.70,0.05,0.08],"core.fringe_hot":[1.0,0.40,0.25],"light.color":[0.95,0.12,0.10],"trail":[0.90,0.10,0.10]}

float ae_Ey(){ return uLength*0.56; }
const float AE_H = 6.0;
float ae_top(){ return (uLength + 0.3)*clamp(uAge/0.45, 0.0, 1.0); }
// how open the eye is: peels open, stays, blinks, closes; flinches on a clash
float ae_open(){
  float p = mod(uTime - 1.5, 16.0);
  float o = smoothstep(-0.2, 1.4, p)*(1.0 - smoothstep(12.0, 13.0, p));
  float bl = 1.0 - (1.0 - smoothstep(0.0, 0.10, abs(p - 7.0)))*0.97;
  bl *= 1.0 - (1.0 - smoothstep(0.0, 0.10, abs(p - 9.6)))*0.97;
  o *= bl*(1.0 - 0.95*uClash)*smoothstep(0.3, 0.8, uAge);
  return clamp(o*(1.0 + 0.12*uSwing), 0.0, 1.15);
}
// where it looks: saccades along the blade (y) and a little across (x), -1..1
vec2 ae_look(){
  float k = floor(uTime/1.6), f = smoothstep(0.0, 0.18, fract(uTime/1.6));
  vec2 a = vec2(h11(k*3.1) - 0.5, h11(k*7.7) - 0.5)*vec2(1.0, 2.0);
  vec2 c = vec2(h11((k + 1.0)*3.1) - 0.5, h11((k + 1.0)*7.7) - 0.5)*vec2(1.0, 2.0);
  vec2 l = mix(a, c, f);
  return clamp(l + vec2(0.0, uSwing*0.8), -1.0, 1.0);
}
// the slit's half width at y: a thin cut, opening into the almond round the eye
float ae_w(float y){
  float L = uLength;
  float base = 0.40*(1.0 - smoothstep(L - 3.0, L, y)) + 0.04;
  float dy = (y - ae_Ey())/AE_H;
  float alm = 1.38*ae_open()*pow(max(1.0 - dy*dy, 0.0), 0.75);
  float top = ae_top();
  return max(base, alm)*smoothstep(-0.2, 0.25, y)*(1.0 - smoothstep(top - 0.6, top, y));
}
// the abyss and the eye, seen on the cut at b (across, along)
vec3 ae_inside(vec2 b, float px){
  float t = uTime;
  float w = ae_w(b.y);
  // the endless dark: black, a slow crimson haze far down in it, motes sinking away
  vec3 col = vec3(0.012, 0.0, 0.006);
  col += vec3(0.16, 0.0, 0.03)*fbm3(vec2(b.x*1.5, b.y*0.3 + t*0.15))*smoothstep(0.0, 0.5, w - abs(b.x));
  vec2 mp = vec2(b.x*3.0, b.y*0.9 + t*0.6); vec2 mc = floor(mp);
  col += vec3(0.9, 0.3, 0.2)*fillD(length(fract(mp) - h22(mc)) - 0.05, px*3.0)*step(0.86, h21(mc))*0.5*(0.5 + 0.5*sin(t*2.0 + mc.y));
  // the eye
  float op = ae_open();
  if (op < 0.01) return col;
  vec2 lk = ae_look();
  vec2 ec = vec2(0.0, ae_Ey());
  vec2 ic = ec + vec2(lk.x*0.22, lk.y*2.6);
  vec2 q = b - ic;
  float roll = lk.y*2.6/AE_H;
  q.y /= mix(1.0, 0.72, abs(roll));                       // the iris foreshortens as it rolls away
  float Ri = 1.15;
  float d = length(q);
  // a dark, veined sclera in the open almond
  float ey = (b.y - ec.y)/AE_H;
  float sclera = smoothstep(0.0, 0.25, op)*(1.0 - smoothstep(0.85, 1.0, abs(ey)));
  float vein = halo(abs(vn(vec2(b.x*3.0, b.y*1.4)) - 0.5), 0.03)*0.6;
  col = mix(col, vec3(0.20, 0.03, 0.03) + vec3(0.45, 0.04, 0.04)*vein, sclera*0.85);
  // the iris: burning amber, fibres radiating, a dark limbal ring
  float ang = atan(q.y, q.x);
  float fib = 0.6 + 0.4*vn(vec2(ang*9.0, d*3.0 - t*0.3));
  vec3 iris = mix(vec3(1.0, 0.85, 0.30), vec3(0.95, 0.35, 0.05), smoothstep(0.2, 0.95, d/Ri))*fib;
  iris += vec3(1.0, 0.9, 0.5)*halo(abs(d - 0.42*Ri), 0.08)*0.5;
  iris *= 1.0 - 0.75*smoothstep(0.82, 1.0, d/Ri);
  float irisA = fillD(d - Ri, px)*sclera;
  col = mix(col, iris, irisA);
  // the eye keeps its own burning amber whatever colour its wearer chose (Creyon's round 9)
  uOwn = max(uOwn, irisA);
  // the bar pupil: wide across, narrow along, narrowing in light and on swings
  float ph = mix(0.30, 0.10, clamp(uSwing*1.5 + 0.4*sin(t*0.5)*0.5 + 0.2, 0.0, 1.0));
  float pup = sdEll(q, vec2(0.66*Ri, ph));
  col = mix(col, vec3(0.0), fillD(pup, px)*irisA);
  // the wet glint
  col += vec3(1.0, 0.95, 0.9)*fillD(length(q - vec2(-0.35, 0.45)) - 0.11, px)*irisA*0.9;
  // the lids' shadow at the almond's edge
  col *= mix(1.0, 0.35, smoothstep(w - 0.25, w, abs(b.x))*sclera);
  return col;
}
// a shadow tendril i: point at u (0 at the lip .. 1 at the tip)
vec3 ae_tendril(float fi, float u){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = (0.08 + 0.85*fract(fi*0.618 + 0.1))*L;
  float a0 = fi*0.382 + 0.05*sin(t*0.4 + fi);
  float reach = (1.8 + 3.6*h11(fi*3.3))*(0.65 + 0.35*sin(t*0.7 + fi*1.9)) + 1.6*sw + 2.0*cl;
  float r = ae_w(y0) + 0.05 + reach*u;
  // it writhes like smoke: up and down, and side to side round the blade
  float wave = sin(u*5.0 - t*2.2 + fi)*0.7*u + u*sin(t*0.5 + fi)*0.8 + u*u*(h11(fi*5.1) - 0.5)*3.0;
  float curl = u*0.10*sin(u*6.0 - t*1.7 + fi*2.3);
  return around(r, a0 + curl, y0 + wave);
}
// the reality cracks: (start, end, fork) for crack i and its life (grow)
float ae_crack(float fi, out vec3 a, out vec3 c, out vec3 m, out vec3 e, out float seed){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float ph = uTime*0.22 + h11(fi*9.0);
  float cy = floor(ph), f = fract(ph);
  float y0 = (0.1 + 0.8*h11(cy*3.0 + fi))*L;
  float an = h11(cy*5.3 + fi*1.7);
  float grow = smoothstep(0.0, 0.25, f)*(1.0 - smoothstep(0.75, 1.0, f));
  float reach = (3.0 + 2.6*h11(cy*2.0 + fi*5.0))*grow*(1.0 + 0.4*sw + 0.6*cl);
  a = around(ae_w(y0) + 0.05, an, y0);
  c = around(ae_w(y0) + 0.05 + reach, an + 0.06*(h11(cy*4.0 + fi) - 0.5), y0 + (h11(cy*7.0 + fi) - 0.5)*reach*0.9);
  m = mix(a, c, 0.5);
  e = m + (c - a)*0.45 + vec3(0.0, (h11(cy*11.0 + fi) < 0.5 ? -1.0 : 1.0)*reach*0.35, 0.0) + around(reach*0.3, an + 0.25, 0.0);
  seed = cy*13.0 + fi;
  return grow*step(0.05, reach);
}
// the cut, drawn over Sol's glow and core as his solid blades are: black abyss and the eye
// inside it, its lips burning crimson with a white-hot hairline at the edge
vec4 sig_solid(vec4 o, vec2 b, float px){
  float top = ae_top();
  if (top < 0.3) return o;
  float w = ae_w(b.y);
  float cov = fillD(abs(b.x) - w, px)*step(0.015, w);
  if (cov <= 0.0) return o;
  vec3 rgb = ae_inside(b, px);
  float rim = clamp(abs(b.x)/max(w, 0.01), 0.0, 1.0);
  rgb += vec3(1.0, 0.18, 0.1)*pow(rim, 3.0)*1.4 + vec3(1.0, 0.6, 0.45)*pow(rim, 10.0)*1.2;
  return vec4(mix(o.rgb, rgb, cov), max(o.a, cov));
}
// shadow tendrils: dark smoky arms, crimson at the tips, reaching out round the blade in 3D,
// each piece facing the camera
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float L = uLength;
  vec4 col = vec4(0.0);
  float top = ae_top();
  // ---- shadow tendrils: dark smoky arms, crimson at the tips ----
  if (uFx > 0.01 && top > L*0.5){
    vec2 bx = raySpan(ro, rd, 7.0, 0.0, L + 3.0);
    if (bx.y > bx.x){
      for (int i = 0; i < 8; i++){
        float fi = float(i);
        vec3 prev = ae_tendril(fi, 0.0);
        for (int k = 1; k <= 6; k++){
          float u1 = float(k)/6.0;
          vec3 nx = ae_tendril(fi, u1);
          float s;
          float d = raySeg(ro, rd, prev, nx, s);
          float u = u1 - (1.0 - s)/6.0;
          float th = mix(0.42, 0.04, sqrt(u));
          if (d < th*2.0 + px*1.5){
            vec3 q = mix(prev, nx, s);
            float tq = dot(q - ro, rd);
            if (tq > 0.0){
              float core = exp(-pow(d/max(th, px*0.7), 2.0));
              float aura = exp(-pow(d/max(th*2.2, px*1.5), 2.0));
              float fade = 1.0 - smoothstep(0.75, 1.0, u);
              float a = max(core*0.92, aura*0.45)*fade*uFx;
              // smoky shadow at the heart of it, its edges and thinning end smouldering crimson
              vec3 rgb = mix(vec3(0.9, 0.1, 0.07)*(0.5 + 0.9*u), vec3(0.02, 0.0, 0.008), core*(1.0 - u*u));
              col = vec4(mix(col.rgb, rgb, a), max(col.a, a));
            }
          }
          prev = nx;
        }
      }
    }
  }
  return col;
}
vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 red = vec3(1.0, 0.15, 0.1);
  float top = ae_top();
  // The blade's own light is Sol's glow and core from the file (sig_glow, sig_core).
  float s;
  float o = ae_open();
  vec3 light = vec3(0.0);
  if (uFx > 0.01 && top > L*0.5){
    vec3 fx = vec3(0.0);
    // crimson tears dripping off the lips and falling
    for (int i = 0; i < 8; i++){
      float fi = float(i);
      float rate = 0.35 + 0.2*h11(fi*2.1);
      float f = fract(t*rate + h11(fi*5.5));
      float cyc = floor(t*rate + h11(fi*5.5));
      float an = h11(fi*7.0 + cyc*1.3);
      float y0 = (0.15 + 0.75*h11(fi*3.7 + cyc))*L;
      vec3 p = around(ae_w(y0) + 0.1 + 0.6*f, an, y0 - 7.0*f*f);
      float d = raySeg(ro, rd, p, p + vec3(0.0, 0.25 + 1.2*f, 0.0), s);
      fx += (vec3(1.0, 0.5, 0.35)*halo(d, max(0.06, px))*1.4 + red*halo(d, 0.3)*0.4)*(1.0 - f)*smoothstep(0.0, 0.08, f);
    }
    // specks of light sucked in from all round
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float f = fract(t*0.25 + fi/12.0);
      float an = fi*0.137 + t*0.05 + f*0.4;
      vec3 p = around(5.2*(1.0 - f) + 0.5, an, (0.2 + 0.6*h11(fi))*L + 2.5*(1.0 - f)*(h11(fi*3.0) - 0.5));
      float tq;
      float d = rayPt(ro, rd, p, tq);
      fx += mix(vec3(0.9, 0.85, 1.0), red, f)*halo(d, max(0.07, px))*1.6*smoothstep(0.0, 0.2, f);
    }
    // cracks in reality: jagged fissures tearing outward round the blade, pulsing with the
    // abyss's heartbeat, brightest where the eye is looking
    float beat = 0.6 + 0.4*pow(0.5 + 0.5*sin(t*2.2), 8.0);
    float gazeY = ae_Ey() + ae_look().y*2.6;
    for (int i = 0; i < 4; i++){
      vec3 a, c, m, e; float seed;
      float live = ae_crack(float(i), a, c, m, e, seed);
      if (live <= 0.0) continue;
      float d = bolt3(ro, rd, a, c, seed, 0.3);
      d = min(d, bolt3(ro, rd, m, e, seed + 3.0, 0.35));
      float nr = 0.5 + 0.9*halo(abs(a.y - gazeY), 5.0)*o;
      vec3 cc = vec3(1.0, 0.55, 0.4)*halo(d, max(px, 0.03))*1.2 + red*halo(d, 0.2)*1.2 + vec3(0.6, 0.0, 0.05)*halo(d, 0.9)*0.45;
      fx += cc*live*beat*nr;
    }
    // clash: a crimson shock ring and a flash
    if (cl > 0.01){
      vec3 h = rayAt(ro, rd, L*0.55);
      if (h.z > 0.0) fx += red*halo(abs(length(h.xy) - 1.0 - 6.0*(1.0 - cl)), 0.18 + 0.2*(1.0 - cl))*cl*1.6;
      float tq;
      fx += red*halo(rayPt(ro, rd, vec3(0.0, L*0.55, 0.0), tq), 1.2)*pow(cl, 3.0);
    }
    light += fx*uFx;
  }
  return light;
}
// the blade: Sol's glow from the file, pulsing slowly; its core is hidden under the cut
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return g*(0.85 + 0.15*sin(uTime*1.7 - b.y*0.2)); }
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }

// ---- round 9: the big effects layer ----
// Creyon's round 9: every mythical dramatic, with big effects all round it in 3D (lab.glsl fx*).
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);

  vec3 red = vec3(1.0, 0.1, 0.06);
  fx += fxArcs(ro, rd, px, y0, y1, 5.0 + 3.0*sw, 4.5 + 2.0*sw, 1.6 + 2.0*sw, 3.1, red);
  fx += fxCoils(ro, rd, px, y0, y1, 1.6, 1.3, 0.5, 2.0, red*0.9);
  fx += fxRays(ro, rd, px, y0, y1, 9.0, 6.0 + 2.0*sw, 0.7, 5.3, red);
  fx += fxMotes(ro, rd, px, y0, y1, 1.2, 5.0, 22.0, 0.08, 1.7, vec3(1.0, 0.25, 0.12));
  fx += fxAura(ro, rd, y0, y1, 2.6, red)*(0.8 + 0.4*sw);
  fx += fxShock(ro, rd, px, L*0.56, 1.0 - cl, 9.0, red)*1.5;
  return col + fx*uFx;
}
