// Abyssal Eye: the blade is a slit cut into an endless dark, its lips glowing crimson, faint motes
// drifting down into the black. Along its middle the slit peels open into a great almond and an eye
// inside looks out: a burning amber iris with a goat's bar pupil, rolling to watch along the blade,
// darting, blinking, narrowing when you swing. On a clash it flinches shut.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.30,0.0,0.04],"corona.rim_hot":[0.95,0.12,0.10],"corona.inner":[1.0,0.45,0.30],"core.white":[1.0,0.70,0.55],"core.fringe_cool":[0.70,0.05,0.08],"core.fringe_hot":[1.0,0.40,0.25],"light.color":[0.95,0.12,0.10],"trail":[0.90,0.10,0.10]}

float ae_Ey(){ return uLength*0.56; }
const float AE_H = 6.0;
// how open the eye is: peels open, stays, blinks, closes; flinches on a clash
float ae_open(){
  float p = mod(uTime - 1.5, 16.0);
  float o = smoothstep(-0.2, 1.4, p)*(1.0 - smoothstep(12.0, 13.0, p));
  float bl = 1.0 - (1.0 - smoothstep(0.0, 0.10, abs(p - 7.0)))*0.97;
  bl *= 1.0 - (1.0 - smoothstep(0.0, 0.10, abs(p - 9.6)))*0.97;
  o *= bl*(1.0 - 0.95*uClash);
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
  return max(base, alm)*smoothstep(-0.1, 0.5, y)*step(y, L);
}
vec3 abyssal_eye_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float w = ae_w(b.y);
  float ed = abs(b.x) - w;
  float on = smoothstep(-0.2, 0.4, b.y)*(1.0 - smoothstep(L - 0.1, L + 0.5, b.y));
  float o = ae_open();
  float near = halo(abs((b.y - ae_Ey())/AE_H), 0.8);
  // the glowing lips of the cut
  vec3 lip = vec3(1.0, 0.25, 0.15)*halo(max(ed, 0.0), 0.06 + px)*1.6 + vec3(1.0, 0.55, 0.4)*halo(max(ed, 0.0), 0.02 + px)*(0.6 + 0.8*o*near);
  float pulse = 0.85 + 0.15*sin(uTime*1.7 - b.y*0.2);
  vec3 glow = vec3(0.85, 0.06, 0.06)*halo(max(ed, 0.0), 0.7)*0.7*pulse + vec3(0.45, 0.0, 0.05)*halo(max(ed, 0.0), 1.8)*0.35;
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  return (lip + glow)*on + vec3(0.5, 0.02, 0.04)*halo(bd, 2.0)*0.12 + g*0.03;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  float w = ae_w(b.y);
  float inside = fillD(abs(b.x) - w, px)*smoothstep(-0.1, 0.3, b.y);
  if (inside <= 0.0) return o;
  float t = uTime;
  // the endless dark: black, a slow crimson haze far down in it, motes sinking away
  vec3 col = vec3(0.012, 0.0, 0.006);
  col += vec3(0.16, 0.0, 0.03)*fbm3(vec2(b.x*1.5, b.y*0.3 + t*0.15))*smoothstep(0.0, 0.5, w - abs(b.x));
  vec2 mp = vec2(b.x*3.0, b.y*0.9 + t*0.6); vec2 mc = floor(mp);
  col += vec3(0.9, 0.3, 0.2)*fillD(length(fract(mp) - h22(mc)) - 0.05, px*3.0)*step(0.86, h21(mc))*0.5*(0.5 + 0.5*sin(t*2.0 + mc.y));
  // the eye
  float op = ae_open();
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
  vec3 iris = mix(vec3(1.0, 0.85, 0.30), vec3(0.95, 0.35, 0.05), smoothstep(0.2, 0.95, d/Ri))*fib*1.0;
  iris += vec3(1.0, 0.9, 0.5)*halo(abs(d - 0.42*Ri), 0.08)*0.5;
  iris *= 1.0 - 0.75*smoothstep(0.82, 1.0, d/Ri);
  float irisA = fillD(d - Ri, px)*sclera;
  col = mix(col, iris, irisA);
  // the bar pupil: wide across, narrow along, narrowing in light and on swings
  float ph = mix(0.30, 0.10, clamp(uSwing*1.5 + 0.4*sin(t*0.5)*0.5 + 0.2, 0.0, 1.0));
  float pup = sdEll(q, vec2(0.66*Ri, ph));
  col = mix(col, vec3(0.0), fillD(pup, px)*irisA);
  // the wet glint
  col += vec3(1.0, 0.95, 0.9)*fillD(length(q - vec2(-0.35, 0.45)) - 0.11, px)*irisA*0.9;
  // the lids' shadow at the almond's edge
  col *= mix(1.0, 0.35, smoothstep(w - 0.25, w, abs(b.x))*sclera);
  return vec4(mix(o.rgb, col, inside), max(o.a, inside));
}

// ==== show (added) ====
// Abyssal Eye show: shadow tendrils with crimson tips reach off the slit and curl back, crimson
// tears drip off it and fall, dark motes and specks of light are sucked in from all round, and
// glowing sigils flicker into the dark and fade. A swing lashes the tendrils out; a clash flares a
// crimson shock ring with the tendrils whipping wide.
vec3 abyssal_eye_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 red = vec3(1.0, 0.15, 0.1), ember = vec3(1.0, 0.55, 0.2);
  vec3 light = vec3(0.0);
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.08 + 0.85*fract(fi*0.618 + 0.1))*L;
    float reach = (3.0 + 2.5*h11(fi*3.3))*(0.6 + 0.4*sin(uTime*0.7 + fi*1.9)) + 3.0*sw + 3.0*cl;
    float u = (b.x*s - 0.3)/max(reach, 0.1);
    if (u < 0.0 || u > 1.0) continue;
    float wave = sin(u*5.0 - uTime*2.5 + fi)*1.2*u + u*u*1.5*sin(uTime*0.5 + fi);
    float dy = b.y - y0 - wave;
    float th = mix(0.28, 0.03, u);
    float t1 = exp(-pow(dy/th, 2.0));
    light += mix(vec3(0.25, 0.02, 0.05), red, u*u)*t1*1.2 + red*halo(length(vec2(u - 1.0, dy/reach)*reach), 0.12)*1.5;
  }
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float f = fract(uTime*(0.35 + 0.2*h11(fi*2.1)) + h11(fi*5.5));
    float s = h11(fi*7.0 + floor(uTime*0.35)) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(0.4 + 0.6*f), (0.15 + 0.75*h11(fi*3.7))*L - 7.0*f*f);
    float dd = length((b - p)*vec2(1.0, 0.6));
    light += red*(fillD(dd - 0.12, px)*1.4 + halo(dd, 0.35)*0.3)*(1.0 - f);
  }
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float f = fract(uTime*0.25 + fi/12.0);
    float a = fi*0.52 + uTime*0.2;
    vec2 p = vec2(cos(a), sin(a)*0.8)*(8.0*(1.0 - f) + 0.6) + vec2(0.0, (0.2 + 0.6*h11(fi))*L);
    light += mix(vec3(0.9, 0.85, 1.0), red, f)*halo(length(b - p), 0.07)*1.6*smoothstep(0.0, 0.2, f);
  }
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float ph = uTime*0.3 + h11(fi*9.0);
    float cy = floor(ph), f = fract(ph);
    vec2 c = vec2((h11(cy + fi*3.0) < 0.5 ? -1.0 : 1.0)*(3.5 + 2.5*h11(cy*2.0 + fi)), (0.15 + 0.7*h11(cy*3.0 + fi))*L);
    vec2 q = b - c;
    float r = length(q), a = atan(q.y, q.x);
    float sig = min(abs(r - 0.9), max(abs(r - 0.5), abs(sin(a*3.0 + cy))*r - 0.04));
    sig = min(sig, abs(q.x*0.7 + q.y*0.7 - 0.0) + step(0.9, r)*9.0);
    float fl = smoothstep(0.0, 0.2, f)*(1.0 - smoothstep(0.6, 1.0, f))*(0.7 + 0.3*step(0.3, h11(floor(uTime*12.0) + fi)));
    light += ember*halo(sig, 0.04 + px)*fl*1.3;
  }
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.55);
    light += red*halo(abs(length(b - o) - 1.0 - 7.0*(1.0 - cl)), 0.15)*cl*1.5;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return abyssal_eye_glow0(g, b, r, px) + abyssal_eye_show(b, min(px, 0.06));
}
