// Mortis: the realm of the Force wielders, the Father, the Daughter and the Son, as a blade
// (Creyon's round 9, rebuilt as Sol builds his). The blade is Sol's glow and core from the file,
// and over it Mortis turns from day to night and back: by day it glows pale and serene, by night
// a deep crimson. Round it in 3D stand two ghost blades at other angles, the Daughter's light and
// the Son's dark, slowly turning round it like the two sides held in check by the Father. By day
// motes of the Daughter's light rise round it and soft beams shine out; by night the Son's red
// lightning crackles off it and embers sink. A swing pulls the ghost blades in to the blade; a
// clash flashes all three and throws a ring of light.
// @part Ghost blades
// @part The Daughter's light
// @part The Son's lightning
// @part The Father's ring
// @room 10
// @palette {"corona.rim_cool":[0.4,0.35,1.0],"corona.rim_hot":[0.7,0.5,1.0],"corona.inner":[0.5,0.8,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.5,0.6,1.0],"core.fringe_hot":[0.8,0.9,1.0],"light.color":[0.45,0.75,1.0],"trail":[0.5,0.7,1.0]}

const vec3 MO_DAY = vec3(0.6, 1.0, 0.8);
const vec3 MO_NIGHT = vec3(1.0, 0.1, 0.08);
// day (1) and night (0): 16 seconds each way, a slow dusk and dawn between
float mo_day(){ return smoothstep(-0.35, 0.35, sin(uTime*6.2832/32.0)); }

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 col = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > 10.0) return col;
  float day = mo_day(), s;
  // ---- the ghost blades: the Daughter's and the Son's, tilted, turning round the blade ----
  for (int j = 0; j < 2; j++){
    float fj = float(j);
    float a = t*0.04 + fj*0.5;
    float tilt = (0.32 - 0.24*sw)*(1.0 - cl);
    vec3 pivot = vec3(0.0, L*0.35, 0.0);
    vec3 dir = normalize(vec3(sin(6.2832*a)*tilt, 1.0, cos(6.2832*a)*tilt));
    vec3 a0 = pivot + dir*1.5, a1 = pivot + dir*(L*0.62);
    float d = raySeg(ro, rd, a0, a1, s);
    float ends = smoothstep(0.0, 0.15, s)*(1.0 - smoothstep(0.85, 1.0, s));
    vec3 gc = j == 0 ? MO_DAY : MO_NIGHT;
    float lit = j == 0 ? 0.35 + 0.65*day : 0.35 + 0.65*(1.0 - day);
    col += (vec3(1.0)*halo(d, max(0.08, px))*0.7 + gc*halo(d, 0.4)*0.6 + gc*halo(d, 1.4)*0.12)*ends*lit*(0.8 + 1.5*cl)*part(0.0);
  }
  // ---- by day: the Daughter's light rising, soft beams ----
  col += (fxMotes(ro, rd, px, y0, y1, 1.0, 6.0, 36.0, 0.06, 5.5, MO_DAY) + fxAura(ro, rd, y0, y1, 3.0, MO_DAY)*0.6)*day*part(1.0);
  // ---- by night: the Son's lightning and sinking embers ----
  col += (fxArcs(ro, rd, px, y0, y1, 5.0, 5.0, 1.8, 2.6, MO_NIGHT) + fxMotes(ro, rd, px, y0, y1, 1.0, 5.0, 20.0, -0.07, 8.8, vec3(1.0, 0.3, 0.15)))*(1.0 - day)*part(2.0);
  // the Father's balance: a slow ring turning round the blade, half day, half night
  {
    vec3 h = rayAt(ro, rd, L*0.78 + sin(t*0.4));
    if (h.z > 0.0){
      float d = abs(length(h.xy) - 3.2);
      float side = 0.5 + 0.5*sin(atan(h.x, h.y) - t*0.2);
      col += (vec3(1.0)*halo(d, max(0.05, px))*0.8 + mix(MO_NIGHT, MO_DAY, side)*halo(d, 0.3)*0.5)*part(3.0);
    }
  }
  col += fxAura(ro, rd, y0, y1, 3.0, mix(MO_NIGHT, MO_DAY, day));
  col += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 10.0, vec3(0.9, 0.9, 1.0))*1.8;
  return col*uFx;
}

// the blade: Sol's glow and core from the file, pale and serene by day, crimson by night, the
// change sweeping up the blade
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = max(g.r, max(g.g, g.b));
  float day = smoothstep(-0.35, 0.35, sin(uTime*6.2832/32.0 - b.y*0.03));
  vec3 c = mix(MO_NIGHT*1.2, mix(MO_DAY, vec3(1.0), 0.25), day);
  return l*c*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
