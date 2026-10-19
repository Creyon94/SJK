// Quasar: a galaxy's engine drawn as a blade. The blade is a thin white-violet relativistic jet with
// bright knots racing up it, wrapped in a twisting violet magnetic helix. Round it floats a tilted
// accretion disk, a ring of hot plasma spiralling in (brighter on the side coming toward you), with
// streams of matter peeling off and falling inward. Knots throw bow-shock arcs as they fly, and
// glowing gas blobs are flung off sideways. A swing flares the jet and flings matter; a clash
// fires a giant knot with a shockwave ring off the disk.
// @room 8
// @palette {"corona.rim_cool":[0.35,0.15,0.85],"corona.rim_hot":[0.75,0.55,1.0],"corona.inner":[0.95,0.90,1.0],"core.white":[0.97,0.95,1.0],"core.fringe_cool":[0.55,0.35,1.0],"core.fringe_hot":[0.85,0.75,1.0],"light.color":[0.6,0.45,1.0],"trail":[0.65,0.5,1.0]}

float qs_d(vec2 b){ return length(vec2(b.x, max(b.y - (uLength - 0.3), 0.0))); }
// the disk: centre height, tilt (the ring's height as a fraction of its width), radius
float qs_dy(){ return uLength*0.3 + 1.5*sin(uTime*0.25); }
const float QS_T = 0.3, QS_R = 4.6;
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float on = smoothstep(-0.5, 0.3, b.y);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float d = qs_d(b);
  vec3 vio = vec3(0.6, 0.38, 1.0), hot = vec3(1.0, 0.7, 0.4), white = vec3(0.95, 0.93, 1.0);
  // ---- the jet: a thin white core in a violet sheath, pulsing up
  float pulse = 0.75 + 0.25*sin(b.y*0.8 - uTime*11.0);
  vec3 light = white*exp(-pow(d/0.3, 2.0))*1.5*on + vio*halo(d, 0.75)*0.55*pulse*(1.0 + 0.6*sw)*on + vio*halo(d, 2.0)*0.08*on;
  // knots racing up the jet, each throwing a bow-shock arc out to both sides
  for (int i = 0; i < 5; i++){
    float fi = float(i);
    float y = mod(uTime*(16.0) + fi*L/5.0, L + 6.0) - 2.0;
    float fade = smoothstep(-1.0, 1.0, y)*(1.0 - smoothstep(L - 2.0, L, y));
    light += white*halo(length(vec2(b.x, (b.y - y)*0.4)), 0.35)*2.0*fade;
    // the bow shock: a chevron of light trailing back from the knot
    float ax = abs(b.x);
    float bs = abs(b.y - (y - 0.55*ax - 0.06*ax*ax));
    light += vio*halo(bs, 0.06 + 0.03*ax)*exp(-ax/1.6)*smoothstep(0.3, 0.9, ax)*fade*1.1;
  }
  // the magnetic helix twisting round the jet
  float inb = step(0.0, b.y)*step(b.y, L - 0.5);
  float ph = b.y*0.9 - uTime*6.0;
  float hx = (0.9 + 0.25*sin(b.y*0.2 + uTime))*sin(ph);
  float front = 0.5 + 0.5*cos(ph);
  light += vio*(halo(abs(b.x - hx), 0.05)*1.8 + halo(abs(b.x - hx), 0.3)*0.2)*(0.3 + 0.7*front)*inb;
  // ---- the accretion disk: a tilted ring round the blade, matter spiralling in
  float dy = qs_dy();
  vec2 q = vec2(b.x, (b.y - dy)/QS_T);
  float rr = length(q);
  float ang = atan(q.y, q.x);
  float behind = step(0.0, q.y);                                  // the far half, hidden by the jet
  float hide = mix(1.0, 1.0 - exp(-pow(b.x/0.5, 2.0)), behind);
  float band = exp(-pow((rr - QS_R)/1.2, 2.0)) + 0.6*exp(-pow((rr - QS_R*0.55)/0.7, 2.0));
  float spiral = 0.55 + 0.45*sin(ang*3.0 + log(max(rr, 0.2))*6.0 + uTime*5.0);
  float doppler = 0.55 + 0.45*cos(ang + 1.57);                    // the approaching side brighter
  float rim = halo(abs(rr - QS_R - 1.1), 0.05);
  vec3 diskC = mix(hot, vec3(1.0, 0.95, 0.85), smoothstep(QS_R, 1.0, rr));
  light += (diskC*band*spiral*(0.35 + 0.9*doppler) + vio*rim*0.6)*hide*(1.0 + 0.7*cl)*(1.0 - smoothstep(QS_R + 1.8, QS_R + 2.6, rr));
  // matter streams peeling off the disk and falling inward to the jet
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float f = fract(uTime*0.5 + fi/6.0);
    float a = fi*1.047 + uTime*1.2 + f*4.0;
    float rad = mix(QS_R + 1.0, 0.4, f*f);
    vec2 p = vec2(rad*cos(a), dy + rad*sin(a)*QS_T + 3.0*f*f);
    light += mix(hot, white, f)*(halo(length(b - p), 0.16 + 0.12*(1.0 - f))*2.2 + halo(length(b - p), 0.6)*0.25)*(0.3 + 0.7*(1.0 - step(0.0, sin(a))*step(abs(p.x), 0.6)));
  }
  // gas blobs flung off the jet sideways, glowing and cooling
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float ph2 = uTime*(0.4 + 0.25*h11(fi*2.3)) + h11(fi*6.1);
    float cy = floor(ph2), f = fract(ph2);
    if (h11(cy*3.7 + fi) > 0.8 + 0.2*sw) continue;
    float s = h11(cy*1.9 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(0.5 + 6.0*f), (0.35 + 0.6*h11(cy*2.7 + fi))*L + 3.0*f);
    float rad = 0.2 + 0.35*f;
    light += mix(white, vio, f)*(halo(length(b - p), rad)*2.0 + halo(length(b - p), rad*3.0)*0.25)*(1.0 - f);
  }
  // clash: a giant knot and a shockwave ring off the disk
  if (cl > 0.01){
    float a = 1.0 - cl;
    float ring = abs(length(vec2(b.x, (b.y - dy)/QS_T)) - QS_R - 5.0*a);
    light += white*(halo(ring, 0.15)*1.6 + halo(ring, 0.6)*0.3)*cl;
    light += white*halo(length(vec2(b.x, (b.y - L*0.6)*0.5)), 1.4)*pow(cl, 3.0)*1.5;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
