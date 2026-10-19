// Wraithlight: a pale haunted blade, like the light of a Force ghost. The blade is translucent and
// cold blue-white, brighter at its edges than its heart, like a glass rod full of moonlight, and
// it shimmers as slow ripples of light pass up it. Cold spirit mist pours off it and sinks toward
// the hilt in soft drifting veils, and faint echoes of the blade itself, its afterglows, hang in
// the air round it, drifting slowly, fading in and out of sight. Swinging leaves more echoes
// behind; a clash makes the blade flare and blows the mist out in a soft ring.
// @room 8
// @palette {"corona.rim_cool":[0.30,0.55,0.95],"corona.rim_hot":[0.72,0.86,1.0],"corona.inner":[0.88,0.95,1.0],"core.white":[0.92,0.97,1.0],"core.fringe_cool":[0.45,0.68,1.0],"core.fringe_hot":[0.78,0.90,1.0],"light.color":[0.55,0.75,1.0],"trail":[0.55,0.75,1.0]}

const vec3 WL_COL = vec3(0.42, 0.66, 1.0);
const vec3 WL_PALE = vec3(0.82, 0.92, 1.0);

float wl_top(){ float g = clamp(uAge/0.45, 0.0, 1.0); return uLength*(1.0 - (1.0 - g)*(1.0 - g)); }
// the translucent blade: brighter at the rim (light passes through more of a shell there)
vec3 wl_body(vec3 ro, vec3 rd, float px, float top, out float dax){
  float s;
  dax = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float y = s*(top + 0.6) - 0.6;
  float rip = 0.5 + 0.5*sin(y*0.45 - uTime*2.2);
  rip = rip*rip;
  float shim = 0.85 + 0.15*sin(y*2.3 + uTime*7.0)*sin(y*0.7 - uTime*3.1);
  float R = 0.42 + 0.04*sin(y*1.1 - uTime*2.6);
  float Rp = max(R, px*1.5);
  float in2 = Rp*Rp - dax*dax;
  // the shell's limb: soft, never a hard line
  float limb = in2 > 0.0 ? Rp/sqrt(in2 + Rp*Rp*0.06) : 0.0;
  limb *= smoothstep(-0.02, 0.06*Rp, in2/Rp);
  limb += exp(-pow((dax - Rp)/(Rp*0.25 + px), 2.0))*2.0;
  float heart = exp(-pow(dax/max(Rp*0.3, px*0.7), 2.0));
  float tipf = 1.0 - smoothstep(top - 1.0, top + 0.2, y);
  vec3 c = WL_PALE*(limb*0.28*(0.8 + 0.6*rip) + heart*0.55)*shim;
  c += WL_COL*halo(dax, 0.9)*(0.45 + 0.35*rip) + WL_COL*halo(dax, 2.6)*0.1;
  return c;
}
// spirit mist density at p: pours off the blade and sinks, spreading as it falls
float wl_mist(vec3 p, float top){
  float y = p.y;
  float r = length(p.xz);
  float a = atan(p.x, p.z)/6.2832;
  float down = clamp(1.0 - y/uLength, 0.0, 1.0);
  float spread = 1.2 + 1.5*down + 1.6*uClash;
  float n = fbm3(vec2(a*4.0 + r*0.3 + uTime*0.03, y*0.11 + uTime*0.5 + r*0.3));
  float n2 = vn(vec2(a*9.0 - uTime*0.05, y*0.22 + uTime*0.8));
  float d = smoothstep(0.38, 0.8, n*0.85 + n2*0.25);
  d *= exp(-pow(r/spread, 2.0))*smoothstep(0.2, 0.6, r + 0.3);
  d *= smoothstep(top + 1.0, top - 2.0, y)*smoothstep(-2.0, 6.0, y);
  return d;
}
// one echo of the blade: a faint copy drifting round it, fading in and out
vec3 wl_echo(vec3 ro, vec3 rd, float i, float px, float top){
  float per = 7.0 + 2.0*h11(i*3.1);
  float ph = fract(uTime/per + h11(i*7.7));
  float life = sin(ph*3.1416); life = life*life;
  float rad = 1.1 + 1.5*h11(i*2.3) + ph*0.9 + uSwing*1.2;
  float ang = h11(i*5.9) + uTime*0.025*(h11(i) > 0.5 ? 1.0 : -1.0) + ph*0.08;
  float y0 = 1.0 + 6.0*h11(i*4.4) + ph*1.5;
  float len = top*(0.5 + 0.35*h11(i*9.1));
  vec3 a = around(rad, ang, y0);
  vec3 b = around(rad + 0.6*(h11(i*1.9) - 0.5), ang + 0.03*(h11(i*6.1) - 0.5), y0 + len);
  float s; float d = raySeg(ro, rd, a, b, s);
  float ends = smoothstep(0.0, 0.25, s)*(1.0 - smoothstep(0.7, 1.0, s));
  float flick = 0.75 + 0.25*sin(uTime*(2.0 + h11(i)*3.0) + s*9.0 - uTime*2.0);
  return (WL_PALE*halo(d, max(px*0.8, 0.07))*0.45 + WL_COL*halo(d, 0.35)*0.22 + WL_COL*halo(d, 1.0)*0.05)*ends*life*flick*(0.9 + 0.7*uSwing);
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float top = wl_top();
  float cl = clamp(uClash, 0.0, 1.0);
  float dax;
  vec3 col = wl_body(ro, rd, px, top, dax)*(1.0 + cl*1.2);
  // spirit mist, marched
  if (dax < 5.5){
    vec3 ms = marchSteps(ro, rd, 5.0, -5.0, top + 1.0, 22.0);
    if (ms.z > 0.0){
      float dt = ms.z;
      float t = ms.x + dt*h21(rd.xy*517.0 + rd.z*91.0);
      float acc = 0.0; float accw = 0.0;
      for (int i = 0; i < 22; i++){
        vec3 p = ro + rd*t;
        float d = wl_mist(p, top);
        float near = exp(-length(p.xz)*0.5);
        acc += d*dt; accw += d*near*dt;
        t += dt;
      }
      float m = acc/(1.0 + acc*0.35);
      float mw = accw/(1.0 + accw*0.35);
      col += (WL_COL*m*0.42 + WL_PALE*mw*0.38)*uFx;
    }
  }
  // echoes of the blade
  if (dax < 5.0){
    vec3 e = vec3(0.0);
    for (int i = 0; i < 6; i++){ e += wl_echo(ro, rd, float(i), px, top); }
    col += e*uFx;
  }
  // clash: a soft ring of mist blown out
  if (cl > 0.0){
    vec3 h = rayAt(ro, rd, uLength*0.55);
    if (h.z > 0.0){
      float rr = length(h.xy), aa = atan(h.x, h.y);
      float R = 0.6 + (1.0 - cl)*4.5;
      float n = 0.6 + 0.4*vn(vec2(aa*3.0, rr*2.0));
      col += WL_PALE*halo(abs(rr - R), 0.6)*cl*n*0.9*uFx;
    }
  }
  return (1.0 - exp(-col*1.1))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
