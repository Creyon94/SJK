// Kyber Heart: a kyber crystal blade that beats like a living heart. The blade is rose-red
// crystal light with a white-hot core, and veins of light wind round it. On every heartbeat,
// lub-dub, a double pulse of light pumps up from the hilt through the veins to the tip: as the
// pulse passes, the blade swells and blazes, and each beat sends a soft shockwave ring rolling
// outward from the base of the blade. Between beats the blade dims and rests. A swing makes
// every beat pump harder and brighter; a clash makes the heart skip: a white flare floods the
// whole blade at once and a strong ring bursts out.
// @room 7
// @palette {"corona.rim_cool":[0.45,0.04,0.18],"corona.rim_hot":[1.0,0.30,0.50],"corona.inner":[1.0,0.80,0.88],"core.white":[1.0,0.94,0.96],"core.fringe_cool":[0.90,0.20,0.45],"core.fringe_hot":[1.0,0.65,0.78],"light.color":[1.0,0.30,0.50],"trail":[1.0,0.32,0.52]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

const float KH_RATE = 1.2;   // beats a second (steady: a changing rate makes the phase race)
const float KH_V = 52.0;     // how fast a pulse climbs the blade (units a second)
// the double beat at phase ph of a beat: lub, then a softer dub
float kh_beatAt(float ph){
  return exp(-pow(ph/0.07, 2.0)) + exp(-pow((ph - 1.0)/0.07, 2.0))
       + 0.65*exp(-pow((ph - 0.24)/0.07, 2.0));
}
// the pulse at height y (it left the hilt y/KH_V seconds ago)
float kh_pulse(float y){
  float ph = fract(uTime*KH_RATE - max(y, 0.0)/KH_V*KH_RATE);
  return kh_beatAt(ph);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 rose = vec3(1.0, 0.28, 0.48), deep = vec3(0.75, 0.05, 0.2), pale = vec3(1.0, 0.78, 0.86), white = vec3(1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fx = uFx;
  float pump = 1.0 + 0.6*sw;
  // ---- the blade: it swells and blazes as each pulse passes, and dims between beats
  float s;
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float y = mix(-0.6, top, s);
  float p = kh_pulse(y)*pump + cl*1.2;
  float w = 0.22 + 0.16*p;
  float pw = mix(p, 0.45*pump + cl, smoothstep(0.4, 2.0, d));
  vec3 light = white*exp(-pow(d/max(w, px), 2.0))*(0.85 + 0.6*p)
             + rose*halo(d, 0.55 + 0.45*p)*(0.6 + 0.9*p)
             + deep*halo(d, 2.0 + 0.9*min(pw, 1.3))*(0.16 + 0.35*min(pw, 1.5));
  if (fx > 0.0){
    vec3 fxl = vec3(0.0);
    float near = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), s);
    // ---- the veins: four strands of light wound round the blade, two each way
    if (near < 1.8){
      for (int v = 0; v < 4; v++){
        float fv = float(v);
        float dir = v < 2 ? 1.0 : -1.0;
        float ph0 = fv*0.5 + (v < 2 ? 0.0 : 0.25);
        vec3 prev = vec3(0.0, -0.4, 0.0);
        float best = 1e3, by = 0.0;
        for (int k = 1; k <= 16; k++){
          float f = float(k)/16.0;
          float yy = mix(-0.4, top, f);
          float pk = kh_pulse(yy)*pump + cl;
          float r = (0.5 + 0.07*sin(yy*0.35 + fv*2.0) + 0.06*pk)*smoothstep(-0.4, 1.5, yy)*(1.0 - smoothstep(top - 2.5, top, yy)*0.8);
          vec3 q = around(r, ph0 + dir*yy*0.036, yy);
          float sv;
          float dv = raySeg(ro, rd, prev, q, sv);
          if (dv < best){ best = dv; by = mix(prev.y, q.y, sv); }
          prev = q;
        }
        float pv = kh_pulse(by)*pump + cl*1.2;
        fxl += (mix(rose, white, min(pv, 1.0))*halo(best, max(px, 0.03))*(0.35 + 1.3*pv) + rose*halo(best, 0.16)*(0.35 + 1.0*pv))*smoothstep(-0.4, 0.5, by);
      }
    }
    // ---- the shockwave rings: one for the lub, one for the dub, rolling out from the base
    float yr = 4.0;
    for (int j = 0; j < 2; j++){
      float fj = float(j);
      float ag = fract(t*KH_RATE - fj*0.24)/KH_RATE;       // seconds since that beat
      float R = 0.6 + ag*7.5;
      float wr = 0.15 + ag*0.6;
      float amp = exp(-ag*2.6)*(1.0 - 0.35*fj)*smoothstep(0.0, 0.05, ag)*pump;
      vec3 o = ro - vec3(0.0, yr + ag*1.5, 0.0); o.y *= 5.0;
      vec3 dd = rd; dd.y *= 5.0; dd = normalize(dd);
      float rr = length(o + dd*max(dot(-o, dd), 0.0));
      float sh = clamp(wr/sqrt(max(R*R - rr*rr, wr*wr)), 0.0, 1.0)*smoothstep(R + wr, R - 2.0*wr, rr) + halo(abs(rr - R), wr*1.4)*0.4;
      fxl += mix(pale, rose, 0.65)*sh*amp*1.0*step(yr, top);
    }
    // ---- a clash: the heart skips, a white flare floods the blade and a strong ring bursts out
    if (cl > 0.01){
      float R = 1.0 + 6.5*(1.0 - cl);
      float wr = 0.3 + 0.5*(1.0 - cl);
      vec3 o = ro - vec3(0.0, L*0.5, 0.0); o.y *= 3.0;
      vec3 dd = rd; dd.y *= 3.0; dd = normalize(dd);
      float rr = length(o + dd*max(dot(-o, dd), 0.0));
      float sh = clamp(wr/sqrt(max(R*R - rr*rr, wr*wr)), 0.0, 1.0)*smoothstep(R + wr, R - 2.0*wr, rr) + halo(abs(rr - R), wr*1.4)*0.5;
      fxl += (pale*sh*1.2)*cl + white*halo(d, 1.5)*pow(cl, 2.0)*0.6;
    }
    light += fxl*fx;
  }
  return (1.0 - exp(-light*1.1))*1.05;
}
