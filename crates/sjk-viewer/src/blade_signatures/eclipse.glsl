// Eclipse: a thin pearl-and-gold blade with a total eclipse floating beside it. The black moon in
// its blazing corona circles the blade (never on it): long pearly streamers, rose prominences and
// flung ejecta; when it passes behind the blade a diamond-ring flash and Baily's beads fire.
// Swing sweeps the streamers back and throws ejecta; a clash flares the corona to full size with a
// giant diamond ring; ignition lifts the eclipse up out of the hilt. Golden coronal loops arch off
// the blade itself and lift away; when the moon crosses the blade (totality) stars come out round it
// and the corona flares out in a ring of long rays.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.75,0.62,0.40],"corona.rim_hot":[1.0,0.90,0.70],"corona.inner":[1.0,0.97,0.90],"core.white":[1.0,0.98,0.92],"core.fringe_cool":[0.95,0.75,0.45],"core.fringe_hot":[1.0,0.92,0.75],"light.color":[1.0,0.86,0.62],"trail":[1.0,0.84,0.58]}

// the eclipse's centre (x, y) beside the blade and z = +1 in front, -1 behind it
float ec_ph(){ return uTime*0.55 + uSeed*6.0; }
vec3 ec_c(){
  float L = uLength, ph = ec_ph();
  float ign = smoothstep(0.0, 0.7, uAge);
  float y = L*(0.58 + 0.22*sin(ph*0.37)) ;
  y = mix(8.0, y, ign);
  float x = 3.4*sin(ph)*mix(0.3, 1.0, ign);
  return vec3(x, y, cos(ph));
}
float ec_my(){ return ec_c().y; }
float ec_R(){ return 1.35; }
// a starburst: soft heart, wide bloom and four thin spikes
vec3 ec_star(vec2 q, float len, float a0){
  float d = length(q);
  float ang = atan(q.y, q.x) + a0;
  float sp = pow(abs(cos(ang*2.0)), 70.0) + 0.5*pow(abs(cos(ang*2.0 + 0.785)), 110.0);
  float rays = sp*exp(-d/len);
  return vec3(1.0, 0.97, 0.92)*(halo(d, 0.25)*2.0 + halo(d, 0.9)*0.5)
       + vec3(0.9, 0.94, 1.0)*rays*2.0;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float ax = abs(b.x), side = sign(b.x + 1e-5);
  float on = smoothstep(-0.8, 0.6, b.y);
  float edge = 1.0 - smoothstep(9.6, 10.8, ax);
  float flare = max(uClash, exp(-uAge*1.4)*step(0.0, uAge)*0.6);

  // ---- the blade body: thin pearl-white core and warm gold glow, ending at the tip
  float yc = clamp(b.y, 0.0, L - 0.4);
  float bd = length(vec2(b.x, b.y - yc));
  float bodyOn = smoothstep(-0.3, 0.4, b.y);
  vec3 C = ec_c();
  float mR = ec_R()*(1.0 + 0.08*flare), mD = length(b - C.xy);
  float shadow = 1.0 - smoothstep(mR + 0.05, mR - 0.15, mD)*step(0.0, C.z);
  vec3 light = vec3(1.0, 0.97, 0.9)*exp(-pow(bd/0.38, 2.0))*1.5*bodyOn;
  light += vec3(1.0, 0.78, 0.45)*halo(bd, 0.8)*0.45*bodyOn;
  light += vec3(1.0, 0.7, 0.4)*halo(bd, 2.4)*0.08*bodyOn;
  light *= shadow;

  // ---- faint corona petals along the whole blade (flickering, swept by swings)
  float x = max(ax - 0.4, 0.0);
  float yb = b.y + x*x*0.1*uSwing*(1.0 + side*0.6);
  float f = vn(vec2(yb*0.4, side*11.0 + uTime*0.25))*0.7 + vn(vec2(yb*1.3, side*3.0 - uTime*0.4))*0.3;
  float pet = smoothstep(0.45, 0.9, f)*exp(-x/(1.5 + uSwing*1.2))*step(0.0, b.y)*(1.0 - smoothstep(L - 2.0, L, b.y));
  light += vec3(1.0, 0.9, 0.75)*pet*0.0;

  // ---- the eclipse: corona ring, streamers, prominences round the moon
  float my = C.y, R = ec_R()*(1.0 + 0.08*flare);
  vec2 q = b - C.xy;
  float d = length(q);
  float ang = atan(q.x, q.y);
  float vis = smoothstep(0.0, 0.4, uAge);
  // behind the blade: the blade's body hides the corona there
  float hide = mix(1.0, 1.0 - exp(-pow(b.x/0.7, 2.0))*step(0.0, b.y)*step(b.y, L), step(C.z, 0.0));
  vis *= hide;
  // the whole corona fades out in a circle, well inside the drawn area (never a square edge)
  vis *= 1.0 - smoothstep(R + 2.6, R + 5.0, d);
  // ring: the chromosphere rim right on the limb
  float rim = exp(-pow((d - R)/0.12, 2.0));
  light += vec3(1.0, 0.95, 0.85)*rim*2.2*vis;
  light += vec3(1.0, 0.35, 0.45)*exp(-pow((d - R - 0.12)/0.08, 2.0))*0.8*vis;
  // inner corona glow
  float out_ = max(d - R, 0.0);
  float reach = 1.0 + 0.6*uSwing + 0.9*flare;
  light += vec3(1.0, 0.93, 0.82)*exp(-out_/(0.45*reach))*0.9*vis*step(R, d);
  // streamers: radial petals tapering to points, slowly turning, swept back on swings
  float sa = ang + 0.25*sin(uTime*0.2) - uSwing*0.5*side*smoothstep(0.0, 4.0, out_);
  float n1 = vn(vec2(sa*2.4, 3.0 + uTime*0.05));
  float n2 = vn(vec2(sa*7.0, 9.0 - uTime*0.08));
  float sharp = mix(0.4, 0.75, smoothstep(0.0, 5.0*reach, out_));
  float strm = smoothstep(sharp, 1.0, n1*0.75 + n2*0.25);
  float fine = pow(vn(vec2(sa*22.0, uTime*0.3)), 4.0);
  float fall = exp(-out_/(2.4*reach)) + 0.25*exp(-out_/(5.0*reach));
  vec3 sc = mix(vec3(1.0, 0.94, 0.82), vec3(0.78, 0.86, 1.0), smoothstep(0.0, 4.0, out_));
  light += sc*(strm*strm*1.8 + fine*2.4)*fall*vis*step(R, d)*(1.0 + 0.8*flare);
  // prominences: rose plasma loops rooted on the limb, rising and erupting
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float cyc = uTime*0.21 + h11(fi*5.7);
    float k = fract(cyc), id = floor(cyc) + fi*13.0;
    float a0 = 6.2832*h11(id*1.9);
    float H = (0.7 + 1.0*h11(id*2.3))*smoothstep(0.0, 0.35, k)*(1.0 + 0.6*flare);
    float w = 0.5 + 0.5*h11(id*3.1);
    float lift = max(k - 0.6, 0.0)*7.0;
    // the loop in the limb's frame: u along the limb, v outward
    float u = (ang - a0); u = mod(u + 3.1416, 6.2832) - 3.1416; u *= R;
    float v = d - R - lift;
    float e = length(vec2(u/w, v/max(H, 0.05)));
    float dd = abs(e - 1.0)*min(w, H) + 1e3*step(v, -0.05);
    float th = 0.1 + 0.07*vn(vec2(u, v)*2.0 + fi);
    float life = smoothstep(0.0, 0.1, k)*(1.0 - smoothstep(0.75, 0.95, k));
    float pr = exp(-pow(dd/th, 2.0)) + halo(dd, 0.35)*0.35;
    light += mix(vec3(0.95, 0.18, 0.38), vec3(1.0, 0.7, 0.78), exp(-pow(dd/(th*0.5), 2.0)))*pr*life*vis*1.1;
  }
  // coronal ejecta: glowing fragments flung off the limb, streaking out and fading (more on swing/clash)
  for (int k = 0; k < 14; k++){
    float fk = float(k);
    float rate = 0.35 + 0.25*h11(fk*2.7);
    float ph = uTime*rate + h11(fk*9.1);
    float cy = floor(ph), f = fract(ph);
    float pa = 6.2832*h11(cy*3.3 + fk*1.7) - uSwing*0.6*side;
    vec2 dir = vec2(sin(pa), cos(pa));
    float rr = R + 0.2 + f*(4.5 + 3.0*h11(cy + fk))*(1.0 + 0.5*flare);
    vec2 pp = C.xy + dir*rr;
    vec2 rel = b - pp;
    float al = dot(rel, dir), pe = dot(rel, vec2(-dir.y, dir.x));
    float tail = 0.4 + f*1.6;
    float sd = length(vec2(pe, al - clamp(al, -tail, 0.0)));
    float fade = (1.0 - f)*smoothstep(0.0, 0.1, f)*step(h11(cy*5.1 + fk), 0.6 + 0.4*max(uSwing, flare));
    vec3 ec = mix(vec3(1.0, 0.92, 0.75), vec3(1.0, 0.45, 0.55), step(0.7, h11(fk*4.4)));
    light += ec*(halo(sd, 0.07)*1.6 + halo(sd, 0.3)*0.25)*fade*vis*smoothstep(-tail, 0.0, al);
  }
  // diamond ring: flashes as the moon enters (lower limb clears the hilt) and leaves (upper limb at tip)
  // the diamond ring fires as the moon's limb crosses the blade (behind it: the blade lights the limb)
  float cross_ = exp(-pow((abs(C.x) - R*0.85)/0.6, 2.0));
  float dr = cross_;
  float enter = step(0.0, C.x*C.z), leave = 1.0 - enter;
  float da = -sign(C.x)*1.5708 + 0.35*sin(uTime*0.7);
  vec2 dpos = C.xy + R*vec2(sin(da), cos(da));
  light += ec_star(b - dpos, 2.2, 0.3 + uTime*0.15)*dr*1.6*vis;
  // a string of Baily's beads along the limb behind the diamond
  for (int j = 0; j < 5; j++){
    float fj = float(j);
    float ba = da + (0.25 + fj*0.22)*sign(C.x + 1e-4);
    vec2 bp = C.xy + R*vec2(sin(ba), cos(ba));
    float fl = 0.6 + 0.4*sin(uTime*11.0 + fj*2.1);
    light += vec3(1.0, 0.97, 0.9)*halo(length(b - bp), 0.12)*1.4*fl*dr*vis*(1.0 - fj*0.15);
  }
  // clash: the corona flares to full size, a giant diamond ring on the limb
  if (flare > 0.01){
    vec2 cp = C.xy + vec2(R*0.7, R*0.7);
    light += ec_star(b - cp, 1.5 + 4.0*flare, 0.4)*flare*1.6;
  }

  // ---- off the blade itself: golden coronal loops arching off it and lifting away
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ph = uTime*(0.32 + 0.15*h11(fi*3.3)) + h11(fi*8.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*2.7 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.12 + 0.72*h11(cy*1.9 + fi*3.0))*L;
    float hw = 1.0 + 1.2*h11(cy + fi*5.0);
    float H = (1.5 + 2.5*h11(cy*4.1 + fi))*smoothstep(0.0, 0.4, f) + 3.0*max(f - 0.6, 0.0);
    vec2 qq = vec2((b.y - y0 - hw)/hw, (b.x*s - 0.35)/max(H, 0.05));
    float ee = length(qq);
    float dd = abs(ee - 1.0)*min(hw, H)*0.8 + 1e3*step(b.x*s, 0.35);
    float life = smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.75, 1.0, f));
    float strand = 0.6 + 0.4*sin(atan(qq.y, qq.x)*10.0 - uTime*3.0 + fi);
    light += mix(vec3(1.0, 0.85, 0.55), vec3(1.0, 0.45, 0.5), f)*(exp(-pow(dd/0.06, 2.0))*1.2 + halo(dd, 0.35)*0.2)*life*strand;
  }
  // ---- stars come out round the blade during totality (when the moon crosses the blade)
  float tot = exp(-pow(C.x/1.6, 2.0))*smoothstep(0.0, 0.5, uAge);
  for (int i = 0; i < 14; i++){
    float fi = float(i);
    vec2 sp = vec2((h11(fi*3.1) - 0.5)*15.0, (0.05 + 0.95*h11(fi*7.3))*L);
    sp.x += sign(sp.x)*1.5;
    float tw = 0.5 + 0.5*sin(uTime*(3.0 + 2.0*h11(fi)) + fi*4.0);
    float sd = length(b - sp);
    float cross_ = halo(abs(b.x - sp.x), 0.025)*halo(abs(b.y - sp.y), 0.35) + halo(abs(b.y - sp.y), 0.025)*halo(abs(b.x - sp.x), 0.35);
    light += vec3(0.85, 0.9, 1.0)*(halo(sd, 0.07)*1.6 + cross_*0.8)*tw*(0.15 + 0.85*tot);
  }
  // ---- totality: as the moon crosses the blade the corona flares out in a ring of long rays
  if (tot > 0.05){
    vec2 q2 = b - C.xy;
    float rr2 = length(q2), a2 = atan(q2.x, q2.y);
    float rays2 = pow(vn(vec2(a2*9.0, floor(uTime*2.0))), 3.0);
    light += vec3(1.0, 0.95, 0.85)*rays2*exp(-max(rr2 - R, 0.0)/4.0)*step(R, rr2)*tot*1.2*(1.0 - smoothstep(R + 4.0, R + 6.0, rr2));
  }
  light *= on*edge;
  light *= 1.0 - smoothstep(L + 0.5, L + 4.5, b.y);
  light *= smoothstep(-7.0, -2.0, b.y);
  return (1.0 - exp(-light*1.1))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  px = min(px, 0.06);
  float L = uLength;
  vec3 C = ec_c();
  float R = ec_R()*(1.0 + 0.08*uClash);
  float vis = smoothstep(0.0, 0.4, uAge);
  float d = length(b - C.xy);
  // behind the blade, the blade's body shows through the moon
  float hide = mix(1.0, 1.0 - fillD(abs(b.x) - 0.42, px)*step(0.0, b.y), step(C.z, 0.0));
  float moon = fillD(d - R + 0.02, px)*vis*hide;
  vec3 face = vec3(0.015, 0.018, 0.03) + vec3(0.05, 0.06, 0.09)*smoothstep(R*0.55, R, d);
  return vec4(face, moon*0.97);
}
