// Royal Naboo: the grace of Naboo's royal court as a blade. A white-gold blade traced with golden
// filigree that slowly flows up it; a queen's golden circlet set with jewels turns slowly round its
// upper part (round 16, Creyon: the scrolls were laggy and did not fit; it is cheap to draw), soft petals of gold light drift down past it and a warm royal glow surrounds it. A
// clash sends a gold ring.
// @part Golden filigree
// @part Petals of light
// @part Royal glow
// @room 8
// @palette {"corona.rim_cool":[0.6,0.45,0.15],"corona.rim_hot":[1.0,0.85,0.5],"corona.inner":[1.0,0.97,0.9],"core.white":[1,1,0.96],"core.fringe_cool":[1.0,0.8,0.45],"core.fringe_hot":[1.0,0.93,0.75],"light.color":[1.0,0.85,0.55],"trail":[1.0,0.85,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float fil = halo(abs(sin(b.y*0.9 - uTime*0.6 + 3.0*sin(b.x*6.0))) - 0.0, 0.12)*part(0.0);
  return recol(g, vec3(1.0, 0.95, 0.82)) + lvl(g)*vec3(1.0, 0.75, 0.3)*fil*0.8;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.78, 0.35), rose = vec3(1.0, 0.7, 0.55);
  fx += fxFlakes(ro, rd, px, y0, y1, 3.5, 12.0, 2.2, 0.4, 0.5, 0.8, 0.8, 4.7, rose)*part(1.0);
  fx += fxAura(ro, rd, y0, y1, 2.4, gold)*0.4*part(2.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 22.0, 0.03, 6.8, vec3(1.0, 0.9, 0.6))*part(2.0);
  return fx*uFx;
}
// round 16: the queen's circlet, a band of gold set with jewels that glint as they come round,
// turning slowly round the blade's upper part, tilted a little; it circles the blade, so it is
// drawn in true 3D (Sol's way); a ring and seven points, cheap to draw
vec3 sig_orbit(vec3 ro, vec3 rd, float px){
  if (part(0.0) < 0.5) { return vec3(0.0); }
  float L = uLength, t = uTime, ta;
  vec3 fx = vec3(0.0);
  float y = L*0.72 + 0.4*sin(t*0.5);
  if (fxNear(ro, rd, y - 3.0, y + 3.0) > 4.5) return fx;
  float R = 1.9;
  vec3 gold = vec3(1.0, 0.78, 0.35);
  float d = ringD(ro, rd, y, R);
  fx += vec3(1.0, 0.9, 0.65)*halo(d, max(0.04, px))*1.2 + gold*halo(d, 0.18)*0.7 + gold*halo(d, 0.6)*0.12;
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    vec3 j = around(R, fi/7.0 + t*0.06, y + (mod(fi, 2.0) < 0.5 ? 0.25 : 0.0));
    float dj = rayPt(ro, rd, j, ta);
    float glint = pow(0.5 + 0.5*sin(t*1.3 + fi*2.4), 8.0);
    vec3 jc = mod(fi, 3.0) < 0.5 ? vec3(1.0, 0.3, 0.4) : (mod(fi, 3.0) < 1.5 ? vec3(0.4, 0.7, 1.0) : vec3(1.0, 0.95, 0.85));
    fx += (vec3(1.0)*halo(dj, max(0.07, px))*(1.0 + 2.0*glint) + jc*halo(dj, 0.25)*0.8);
  }
  return fx*uFx;
}
