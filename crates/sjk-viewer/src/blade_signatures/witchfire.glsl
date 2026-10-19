// Witchfire: the Nightsister magick of Dathomir, as a blade. The blade is a column of sickly green
// witchfire ichor with a pale burning heart, wrapped all round in swirling green ghost-fire whose
// tongues lick up its length. Thick ichor mist pours down off it and spills over the hilt, three
// tendrils of green energy coil round it and slide upward, dark smoke curls through the fire, and
// pulses of magick climb the blade from the hilt to the tip. Swinging feeds the fire; a clash
// blasts it outward in a ring of green flame.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.04,0.35,0.08],"corona.rim_hot":[0.35,1.0,0.35],"corona.inner":[0.8,1.0,0.75],"core.white":[0.92,1.0,0.9],"core.fringe_cool":[0.2,0.85,0.3],"core.fringe_hot":[0.7,1.0,0.6],"light.color":[0.3,1.0,0.35],"trail":[0.35,1.0,0.4]}

const vec3 WF_G = vec3(0.10, 1.0, 0.09);
const vec3 WF_DEEP = vec3(0.02, 0.50, 0.05);
const vec3 WF_PALE = vec3(0.78, 1.0, 0.62);

float wf_top(){ float g = clamp(uAge/0.45, 0.0, 1.0); return uLength*(1.0 - (1.0 - g)*(1.0 - g)); }
// pulses of magick climbing the blade
float wf_pulse(float y){
  float f = fract(y*0.045 - uTime*0.7);
  return pow(f, 10.0)*smoothstep(1.0, 0.97, f)*2.0;
}
// ghost-fire and ichor mist at point p: (fire, mist)
vec2 wf_dens(vec3 p, float top){
  float y = p.y;
  float r = length(p.xz);
  float a = atan(p.x, p.z)/6.2832;
  float hot = 1.0 + uSwing*0.3 + uClash*0.45;
  // the fire: tongues swirling round and racing up
  float sw = a + y*0.035 + uTime*0.22;
  float n = fbm3(vec2(sw*5.0, y*0.42 - uTime*2.6));
  float n2 = vn(vec2(sw*11.0, y*0.9 - uTime*4.1));
  float reach = (0.9 + 3.4*n*n + 0.8*n2)*hot;
  float fire = pow(smoothstep(reach, 0.3, r), 1.5)*smoothstep(0.28, 0.6, n + 0.15*n2);
  fire *= smoothstep(-0.4, 1.0, y)*(1.0 - smoothstep(top - 1.5, top + 1.5, y));
  // the ichor mist: pours down, thickens and spreads toward the hilt
  float sp = 1.0 - clamp(y/uLength, 0.0, 1.0);
  float m = fbm3(vec2(a*4.0 - uTime*0.05, y*0.28 + uTime*1.5));
  float mr = 0.9 + 2.6*sp*sp + 1.0*m;
  float mist = smoothstep(mr, 0.2, r)*smoothstep(0.35, 0.75, m)*smoothstep(top + 0.5, top - 3.0, y)*smoothstep(-4.0, 0.0, y);
  return vec2(fire, mist*0.55);
}
// one coiling tendril of green energy
vec3 wf_tendril(vec3 ro, vec3 rd, float k, float px, float top){
  float best = 1e3; float bs = 0.0; float s;
  vec3 prev = vec3(0.0);
  float ph = k/3.0 + uTime*0.12;
  float slide = uTime*0.9 + k*4.0;
  for (int i = 0; i <= 20; i++){
    float f = float(i)/20.0;
    float y = f*top*1.02 - 0.3;
    float rr = 0.9 + 0.8*(0.5 + 0.5*sin(f*7.0 + k*2.1 - uTime*1.3)) + uSwing*0.6;
    vec3 q = around(rr, ph + f*(1.3 + 0.25*k), y);
    if (i > 0){
      float d = raySeg(ro, rd, prev, q, s);
      if (d < best){ best = d; bs = (float(i) - 1.0 + s)/20.0; }
    }
    prev = q;
  }
  // energy sliding along it
  float flow = 0.45 + 0.55*pow(0.5 + 0.5*sin((bs*top - slide*6.0)*0.35), 3.0);
  float fade = smoothstep(0.0, 0.08, bs)*(1.0 - smoothstep(0.88, 1.0, bs));
  return (WF_PALE*halo(best, max(px, 0.03))*0.6 + WF_G*halo(best, 0.14)*0.8 + WF_G*halo(best, 0.6)*0.18)*flow*fade;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float top = wf_top();
  float cl = clamp(uClash, 0.0, 1.0);
  vec3 col = vec3(0.0);
  // the ichor blade itself: pale burning heart, sickly green body, pulses climbing it
  float sa;
  float dax = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), sa);
  float yb = sa*(top + 0.6) - 0.6;
  float pu = wf_pulse(yb);
  float wob = 0.30 + 0.05*sin(yb*1.7 - uTime*6.0) + 0.06*pu;
  col += WF_PALE*exp(-pow(dax/max(wob*0.45, px*0.7), 2.0))*1.0;
  col += WF_G*halo(dax, wob*1.5)*(0.9 + 0.8*pu);
  col += WF_G*halo(dax, 1.6)*0.25*(1.0 + pu);
  // ghost-fire and ichor mist, marched
  if (dax < 4.2){
    vec3 ms = marchSteps(ro, rd, 4.0, -4.0, top + 2.0, 22.0);
    if (ms.z > 0.0){
      float dt = ms.z;
      float t = ms.x + dt*h21(rd.xy*517.0 + rd.z*91.0);
      vec3 acc = vec3(0.0);
      for (int i = 0; i < 22; i++){
        vec3 p = ro + rd*t;
        vec2 d = wf_dens(p, top);
        float pu2 = wf_pulse(p.y);
        acc += (WF_G*d.x*(1.0 + 1.2*pu2) + WF_PALE*d.x*d.x*d.x*0.2 + WF_DEEP*d.y*2.2)*dt;
        t += dt;
      }
      col += acc*(1.0/(1.0 + length(acc)*0.3))*uFx;
    }
  }
  // tendrils
  if (dax < 3.4){
    vec3 td = vec3(0.0);
    for (int k = 0; k < 3; k++){ td += wf_tendril(ro, rd, float(k), px, top); }
    col += td*uFx;
  }
  // clash: a ring of green flame blasts out
  if (cl > 0.0){
    vec3 h = rayAt(ro, rd, uLength*0.6);
    if (h.z > 0.0){
      float rr = length(h.xy), aa = atan(h.x, h.y);
      float R = 0.5 + (1.0 - cl)*5.0;
      float tongue = 0.6 + 0.4*vn(vec2(aa*4.0, uTime*8.0));
      col += (WF_G*halo(abs(rr - R), 0.5*tongue) + WF_PALE*halo(abs(rr - R), 0.12))*cl*1.5*uFx;
    }
    col += WF_G*halo(dax, 2.5)*cl*0.7;
  }
  return (1.0 - exp(-col*1.1))*1.05;
}
// dark smoke curling through the outer fire
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float top = wf_top();
  vec3 ms = marchSteps(ro, rd, 4.2, 0.0, top + 3.0, 14.0);
  if (ms.z <= 0.0 || uFx < 0.5) return vec4(0.0);
  float dt = ms.z;
  float t = ms.x + dt*h21(rd.xy*311.0 + rd.z*57.0);
  float tr = 1.0;
  for (int i = 0; i < 14; i++){
    vec3 p = ro + rd*t;
    float r = length(p.xz);
    float a = atan(p.x, p.z)/6.2832;
    float n = fbm3(vec2(a*3.0 + p.y*0.05 + uTime*0.1, p.y*0.22 - uTime*0.7));
    float d = smoothstep(0.55, 0.85, n)*smoothstep(1.0, 2.0, r)*smoothstep(4.0, 2.6, r)
      *smoothstep(0.0, 3.0, p.y)*smoothstep(top + 2.0, top - 2.0, p.y);
    tr *= exp(-d*dt*0.35);
    t += dt;
  }
  float al = clamp((1.0 - tr)*0.85, 0.0, 0.55);
  return vec4(vec3(0.01, 0.025, 0.012), al);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
