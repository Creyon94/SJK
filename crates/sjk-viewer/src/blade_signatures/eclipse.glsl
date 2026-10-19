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
  // The blade is Sol's glow and core from the file, dark in the moon's band (sig_glow,
  // sig_core, sig_solid); here only the effects, each a piece placed round it in 3D.
  vec3 light = vec3(0.0);
  vec3 fx = vec3(0.0);
  float vis = smoothstep(0.1, 0.6, uAge);
  // ---- the corona: long pearly streamers fanning out from the dark band in every direction
  float reach = 1.0 + 0.35*sw + 0.45*flare;
  if (rayPt(ro, rd, vec3(0.0, yb, 0.0), tq) < 7.5 && vis > 0.0){
    for (int i = 0; i < 28; i++){
      float fi = float(i);
      float a = fi/28.0 + 0.012*sin(uTime*0.3 + fi) + uTime*0.008;
      float el = (h11(fi*3.7) - 0.5)*0.9 + 0.12*sin(uTime*0.4 + fi*1.3);
      float len = (3.0 + 4.5*pow(h11(fi*5.1), 1.3))*reach*(0.85 + 0.15*sin(uTime*0.7 + fi*2.0));
      vec3 p0 = around(0.5, a, yb + el*0.6);
      vec3 p1 = around(0.5 + len, a + 0.03*sw, yb + el*(0.6 + len*0.7));
      float dd = raySeg(ro, rd, p0, p1, s);
      float taper = 1.0 - s*s;
      float w = max(0.06 + 0.26*(1.0 - s), px);
      fx += mix(white, pearl, s)*(halo(dd, w)*1.2 + halo(dd, w*3.0)*0.25)*taper*(1.0 + 0.6*flare)*vis;
    }
    // the inner corona: a bright soft ring of light hugging the band
    float dc = rayPt(ro, rd, vec3(0.0, yb, 0.0), tq);
    fx += white*(halo(abs(dc - 0.9), 0.35)*0.5 + halo(dc, 1.6)*0.15)*vis*(1.0 + flare);
  }
  // ---- the chromosphere: thin bright rims along both edges of the band, round the blade
  for (int k = 0; k < 2; k++){
    float ey = yb + (float(k)*2.0 - 1.0)*hw*0.62;
    vec3 hp = rayAt(ro, rd, ey);
    if (hp.z > 0.0){
      float rr = length(hp.xy);
      float rim = halo(abs(rr - 0.42), max(0.05, px))*1.1;
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
  return light;
}
// the moon's shadow, drawn over Sol's glow as his solid blades are: the blade's heart goes
// dark across the band
vec4 sig_solid(vec4 o, vec2 b, float px){
  float a = (1.0 - smoothstep(0.22, 0.42, abs(b.x)))*ec_shade(b.y)*smoothstep(0.1, 0.5, uAge);
  return vec4(mix(o.rgb, vec3(0.02, 0.018, 0.015), a*0.9), max(o.a, a*0.9));
}
// the blade: Sol's glow and core from the file, light running up it, out in the band
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float sheen = 0.85 + 0.15*sin(b.y*1.3 - uTime*6.0) + 0.1*sin(b.y*0.37 + uTime*2.3);
  return g*sheen*(1.0 - 0.94*ec_shade(b.y));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*(1.0 - 0.94*ec_shade(b.y)); }
