// Eclipse: a thin pearl-white blade with a total eclipse travelling along it. Where the moon
// passes, a dark ring crosses the blade and its light goes out in a band, and round that band a
// real corona flares out in every direction: long pearly streamers fanning out round the blade,
// brightest close in, shimmering and slowly turning. Seen from above it is a ring of corona rays
// round a dark circle, like a true eclipse. The band drifts slowly up and down the blade, with
// diamond-ring flashes and Baily's beads sparking on its edge, while small pearly prominences
// loop up off the blade and fade. Ignites fast; a swing sweeps the streamers out, a clash flares the
// whole corona with a diamond-ring burst.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.75,0.62,0.40],"corona.rim_hot":[1.0,0.90,0.70],"corona.inner":[1.0,0.97,0.90],"core.white":[1.0,0.98,0.92],"core.fringe_cool":[0.95,0.75,0.45],"core.fringe_hot":[1.0,0.92,0.75],"light.color":[1.0,0.86,0.62],"trail":[1.0,0.84,0.58]}

float ec_top(){ return uLength*clamp(uAge/0.4, 0.0, 1.0); }
// the band's height: drifting up and down the blade, rising out of the hilt at ignition
float ec_y(){
  float L = uLength;
  float y = L*(0.52 + 0.3*sin(uTime*0.3 + uSeed*6.0));
  return mix(7.0, y, smoothstep(0.2, 1.2, uAge));
}
float ec_hw(){ return 1.4; }
float ec_ringR(){ return 0.55; }
// how dark the blade is at height y (1 in the band)
float ec_shade(float y){
  float yb = ec_y(), hw = ec_hw();
  return (1.0 - smoothstep(hw*0.45, hw*0.8, abs(y - yb)))*smoothstep(0.0, 0.5, uAge);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, top = ec_top(), s, tq;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float flare = max(cl, exp(-uAge*1.6)*step(0.0, uAge)*0.35);
  vec3 pearl = vec3(1.0, 0.9, 0.72), white = vec3(1.0, 0.98, 0.94);
  float yb = ec_y(), hw = ec_hw(), R0 = ec_ringR();
  // ---- the blade: a thin pearl core with light running up it, dark in the moon's band
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, max(top, -0.5), 0.0), s);
  float y = mix(-0.6, top, s);
  float sheen = 0.85 + 0.15*sin(y*1.3 - uTime*6.0) + 0.1*sin(y*0.37 + uTime*2.3);
  float shade = ec_shade(y);
  vec3 light = (white*exp(-pow(d/max(0.3, px), 2.0))*1.45 + pearl*halo(d, 0.75)*0.6*sheen + pearl*halo(d, 2.2)*0.12)*(1.0 - 0.94*shade);
  vec3 fx = vec3(0.0);
  float vis = smoothstep(0.1, 0.6, uAge);
  // ---- the corona: a fan of streamers radiating out from the dark ring in 3D
  float Rc = 6.2;
  vec3 m = marchSteps(ro, rd, Rc, yb - 3.2, yb + 3.2, 24.0);
  if (m.y > m.x && vis > 0.0){
    float t = m.x + m.z*h21(rd.xy*517.0);
    float reach = 1.0 + 0.35*sw + 0.45*flare;
    for (int i = 0; i < 24; i++){
      vec3 p = ro + rd*t;
      vec3 q = p - vec3(0.0, yb, 0.0);
      float r = length(q.xz);
      vec2 mer = vec2(r - R0*0.6, q.y);
      float rho = length(mer);
      float el = atan(q.y, max(r - R0*0.6, 0.05));
      // streamers: a pattern of direction only, so it reads as rays fanning outward
      vec2 az = q.xz/max(r, 1e-3);
      vec2 az2 = rot(uTime*0.05 + sw*0.3)*az;
      float n = vn(az2*3.2 + vec2(el*3.0, 0.0) + vec2(3.0, uTime*0.08));
      float n2 = vn(az2*11.0 + vec2(el*8.0, 7.0) - vec2(0.0, uTime*0.15));
      float strm = smoothstep(0.4, 0.9, n*0.6 + n2*0.4);
      float fine = pow(n2, 5.0);
      float fan = exp(-el*el/(0.28 + 0.25*sw)) + 0.25*exp(-pow((abs(el) - 1.3)/0.25, 2.0));
      float fall = exp(-max(rho - 0.3, 0.0)/(1.1*reach)) + 0.4*exp(-max(rho - 0.3, 0.0)/(3.0*reach));
      float inner = exp(-max(rho - 0.3, 0.0)/0.4);
      float dens = (strm*strm*2.2 + fine*1.6 + 0.08)*fall*fan + inner*0.8;
      dens *= smoothstep(0.2, 0.55, rho)*(1.0 - smoothstep(Rc - 2.4, Rc, r))*exp(-q.y*q.y/2.2);
      vec3 c = mix(white, pearl, smoothstep(0.3, 3.0, rho));
      fx += c*dens*m.z*0.9*(1.0 + 0.6*flare);
      t += m.z;
    }
  }
  // ---- the chromosphere: thin bright rims along both edges of the band, round the blade
  for (int k = 0; k < 2; k++){
    float ey = yb + (float(k)*2.0 - 1.0)*hw*0.62;
    vec3 hp = rayAt(ro, rd, ey);
    if (hp.z > 0.0){
      float rr = length(hp.xy);
      float rim = halo(abs(rr - 0.42), max(0.05, px))*1.1 + halo(rr, 0.5)*0.4;
      fx += white*rim*vis*(1.0 - smoothstep(0.6, 1.2, rr));
    }
  }
  // ---- diamond ring and Baily's beads: flashes on the band's edge as the moon slides
  float cyc = uTime*0.45, f = fract(cyc), id = floor(cyc);
  float fl = exp(-f*5.0)*smoothstep(0.0, 0.03, f) + flare*0.8;
  float upper = h11(id*3.7) < 0.5 ? 1.0 : -1.0;
  float a0 = h11(id*1.9);
  vec3 dp = around(0.42, a0, yb + upper*hw*0.62);
  float dd = rayPt(ro, rd, dp, tq);
  fx += white*(halo(dd, 0.12)*2.5 + halo(dd, 0.6)*0.6 + halo(dd, 2.0)*0.15)*fl*vis;
  // a soft horizontal flash streak through the diamond, round the band
  vec3 hp2 = rayAt(ro, rd, dp.y);
  if (hp2.z > 0.0){
    float rr = length(hp2.xy);
    fx += white*halo(abs(rr - 0.42), 0.25)*exp(-rr/2.5)*fl*0.6*vis;
  }
  for (int j = 0; j < 5; j++){
    float fj = float(j);
    vec3 bp = around(0.42, a0 + (fj + 1.0)*0.07*upper, dp.y);
    float db = rayPt(ro, rd, bp, tq);
    float tw = 0.6 + 0.4*sin(uTime*13.0 + fj*2.1);
    fx += white*halo(db, 0.07)*1.6*tw*exp(-f*3.0)*(1.0 - fj*0.15)*vis;
  }
  // ---- prominences: pearly loops of plasma arching off the blade, rising and lifting away
  float dA = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), s);
  if (dA < 4.5){
    for (int i = 0; i < 4; i++){
      float fi = float(i);
      float ph = uTime*(0.25 + 0.1*h11(fi*3.3)) + h11(fi*8.1);
      float cy = floor(ph), fp = fract(ph);
      float y0 = (0.1 + 0.8*h11(cy*1.9 + fi*3.0))*top;
      if (abs(y0 - yb) < hw + 1.0) continue;
      float az = h11(cy*2.7 + fi);
      float span = 0.5 + 0.6*h11(cy + fi*5.0);
      float H = (0.45 + 0.5*h11(cy*4.1 + fi))*smoothstep(0.0, 0.35, fp)*(1.0 + 0.8*max(fp - 0.5, 0.0));
      float life = smoothstep(0.0, 0.1, fp)*(1.0 - smoothstep(0.7, 1.0, fp));
      vec3 prev = around(0.35, az, y0);
      for (int j = 1; j <= 10; j++){
        float u = float(j)/10.0;
        float lift = 0.0;
        vec3 p = around(0.35 + H*sin(u*3.1416) + lift, az + 0.03*sin(u*3.1416*2.0)*H, y0 + span*u*2.0);
        float dh = raySeg(ro, rd, prev, p, s);
        float strand = 0.65 + 0.35*sin((float(j) + s)*2.0 - uTime*4.0 + fi);
        fx += mix(white, pearl, 0.6)*(halo(dh, max(0.06, px))*0.8 + halo(dh, 0.3)*0.4)*life*strand;
        prev = p;
      }
    }
  }
  light += fx*uFx;
  return (1.0 - exp(-light*1.1))*1.05;
}
// the moon's shadow: the blade's heart goes dark across the band (only within the blade's width)
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float yb = ec_y(), hw = ec_hw(), s;
  float d0 = raySeg(ro, rd, vec3(0.0, yb - hw*0.45, 0.0), vec3(0.0, yb + hw*0.45, 0.0), s);
  float a = (1.0 - smoothstep(0.22, 0.42, d0))*smoothstep(0.1, 0.5, uAge);
  return vec4(vec3(0.02, 0.018, 0.015), a*0.9);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
