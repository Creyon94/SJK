// Star Dragon: the power of the Duinuogwuin, the star dragons of legend, held as a blade. No dragon
// is drawn: the blade itself is the dragon. Its body is sheathed in overlapping scales of gold light
// that wrap all the way round it and ripple upward like a living hide; a deep inner fire pulses
// slowly up its heart. Dragon breath of stardust-fire rolls up the blade and peels off it near the
// tip, full of glinting star sparks, and dark-gold smoke curls round the hilt. A swing makes the
// scales flare open and blaze and the breath roar; a clash flashes the whole hide white-hot.
// @room 8
// @palette {"corona.rim_cool":[0.45,0.26,0.04],"corona.rim_hot":[1.0,0.66,0.18],"corona.inner":[1.0,0.88,0.60],"core.white":[1.0,0.97,0.90],"core.fringe_cool":[1.0,0.60,0.15],"core.fringe_hot":[1.0,0.86,0.55],"light.color":[1.0,0.68,0.24],"trail":[1.0,0.70,0.28]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// both crossings of the ray with the cylinder of radius R round the axis (x > y: none)
vec2 sdr_cyl(vec3 ro, vec3 rd, float R){
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float h = b*b - a*c;
  if (a < 1e-6 || h < 0.0) return vec2(1.0, -1.0);
  h = sqrt(h);
  return vec2((-b - h)/a, (-b + h)/a);
}
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
// the breath: soft fire density at p (rolling up and peeling off toward the tip)
float sdr_fire(vec3 p, float roar){
  float L = uLength;
  float r = length(p.xz);
  float yl = clamp(p.y/L, 0.0, 1.2);
  float rc = 0.9 + 2.2*yl*yl + roar*0.8;
  float sh = exp(-pow(max(r - 0.6, 0.0)/rc, 2.0)*2.2)*smoothstep(0.55, 1.0, r);
  float n = fbm3(vec2(p.x*0.75 + p.z*0.25, p.y*0.16 - uTime*1.4))*fbm3(vec2(p.z*0.75 - p.x*0.25, p.y*0.19 - uTime*1.15 + 7.0));
  n = smoothstep(0.26, 0.52, n*1.5);
  float lo = smoothstep(L*0.04, L*0.35, p.y);
  float hi = 1.0 - smoothstep(L*0.97, L*1.12, p.y);
  return sh*n*lo*hi*(1.0 - smoothstep(3.6, 6.2, r));
}
// smoke round the hilt
float sdr_smoke(vec3 p){
  float r = length(p.xz);
  float a = atan(p.x, p.z);
  vec2 q = vec2(cos(a + p.y*0.7 - uTime*0.6), sin(a + p.y*0.7 - uTime*0.6))*r;
  float n = fbm3(q*0.9 + vec2(0.0, -uTime*0.3) + vec2(p.y*0.35));
  float sh = exp(-pow((r - 0.9 - p.y*0.18)/0.9, 2.0))*(1.0 - smoothstep(2.5, 5.5, p.y))*smoothstep(-0.8, 0.4, p.y);
  return sh*smoothstep(0.35, 0.75, n);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 gold = vec3(1.0, 0.66, 0.20);
  vec3 ember = vec3(1.0, 0.45, 0.10);
  vec3 hot = vec3(1.0, 0.90, 0.65);
  float flare = clamp(sw*0.9 + cl, 0.0, 1.0);

  // the body: a white-hot heart with a slow inner fire pulsing up it
  float s;
  float dA = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float ya = mix(-0.6, top, s);
  float pulse = 0.5 + 0.5*sin(ya*0.35 - uTime*2.2);
  pulse = pulse*pulse*pulse;
  float tipF = 1.0 - smoothstep(top - 0.4, top + 0.3, ya);
  vec3 light = vec3(1.0)*exp(-pow(dA/max(0.22, px), 2.0))*(1.25 + 0.35*pulse + cl)
    + mix(gold, hot, 0.3*pulse)*halo(dA, 0.45)*(0.45 + 0.5*pulse + 0.4*flare)
    + gold*halo(dA, 2.2)*(0.16 + 0.10*pulse + 0.15*flare);
  light += ember*halo(dA, 1.0)*0.25*(0.5 + 0.5*sin(ya*1.3 + uTime*5.0 + 2.0*sin(ya*0.4 - uTime)));

  // the scale hide: a shell wrapped round the blade, front and back sheets
  float Rs = 0.85 + 0.2*flare;
  vec2 c2 = sdr_cyl(ro, rd, Rs);
  if (c2.y > c2.x){
    for (int k = 0; k < 2; k++){
      float t = k == 0 ? c2.x : c2.y;
      if (t > 0.0){
        vec3 p = ro + rd*t;
        if (p.y > -0.4 && p.y < top - 0.2){
          vec3 nrm = vec3(p.x, 0.0, p.z)/Rs;
          float cs = abs(dot(nrm, rd));
          float aa = px/0.4;
          float sc = sdr_scale(p, flare, aa);
          float fres = 1.0/max(cs, 0.35);
          float fadeY = smoothstep(-0.4, 0.4, p.y)*(1.0 - smoothstep(top - 1.6, top - 0.2, p.y));
          // a ripple of lifting scales running up the hide
          float rip = smoothstep(0.75, 1.0, sin(p.y*0.25 - uTime*1.8));
          float w = (k == 0 ? 0.55 : 0.30)*fadeY*fres*(0.8 + 0.6*rip + 0.4*pulse);
          light += mix(gold, hot, clamp(sc - 0.8, 0.0, 1.0) + cl*0.6)*sc*w;
        }
      }
    }
  }

  // dragon breath: a marched volume of stardust-fire round the upper blade
  float roar = sw*0.8 + cl*0.5;
  vec3 ms = marchSteps(ro, rd, 6.3, L*0.03, L*1.12, 22.0);
  if (ms.y > ms.x && fxOn > 0.001){
    float t = ms.x + ms.z*h21(rd.xy*517.0 + rd.z*91.0);
    vec3 acc = vec3(0.0);
    for (int i = 0; i < 22; i++){
      vec3 p = ro + rd*t;
      if (p.y < top){
        float d = sdr_fire(p, roar);
        float r = length(p.xz);
        float heat = exp(-r*0.55)*(0.7 + 0.3*d);
        acc += mix(ember, mix(gold, hot, heat), 0.4 + 0.6*heat)*d*ms.z;
      }
      t += ms.z;
    }
    light += acc*(0.75 + 0.3*roar)*fxOn;
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

  // smoke curling at the hilt (dim warm haze, lit by the blade)
  vec3 mk = marchSteps(ro, rd, 3.2, -0.8, 6.0, 12.0);
  if (mk.y > mk.x && fxOn > 0.001){
    float t = mk.x + mk.z*h21(rd.xy*311.0 + rd.z*13.0);
    float acc = 0.0;
    for (int i = 0; i < 12; i++){
      vec3 p = ro + rd*t;
      acc += sdr_smoke(p)*mk.z;
      t += mk.z;
    }
    light += vec3(0.55, 0.38, 0.18)*acc*0.28*fxOn;
  }

  // a clash: a burst of light off the hide
  light += hot*halo(dA, 3.0)*cl*0.5*fxOn;
  return light;
}
