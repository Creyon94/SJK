// Royal Naboo: the grace of Naboo's royal court as a blade. A white-gold blade traced with golden
// filigree that slowly flows up it; golden scrolls of embroidery curl open beside it in pairs, one
// above another, and fade (round 15: they replaced ribbons it shared with the Aurodium), soft petals of gold light drift down past it and a warm royal glow surrounds it. A
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
  // golden scrolls: mirrored curls of embroidery opening beside the blade, one pair at a time
  float s;
  if (fxNear(ro, rd, y0, y1) < 4.5){
    for (int j = 0; j < 4; j++){
      float fj = float(j);
      float q = fract(t*0.18 - fj/4.0);
      float yc = mix(y0 + 2.0, y1 - 2.0, fj/3.0);
      float grow = smoothstep(0.0, 0.45, q), fade = 1.0 - smoothstep(0.7, 1.0, q);
      for (int m = 0; m < 2; m++){
        float side = m == 0 ? -1.0 : 1.0;
        vec3 prev = vec3(side*0.6, yc, 0.0);
        float best = 1e3;
        for (int i = 1; i <= 12; i++){
          float u = float(i)/12.0;
          if (u > grow) break;
          float a = u*7.5;
          float r = 1.5*(1.0 - 0.75*u);
          vec3 p = vec3(side*(0.6 + 1.5 + r*sin(a - 1.5708)), yc + r*cos(a - 1.5708) + 0.8*u, 0.0);
          best = min(best, raySeg(ro, rd, prev, p, s));
          prev = p;
        }
        fx += (vec3(1.0, 0.95, 0.8)*halo(best, max(0.025, px))*1.1 + gold*halo(best, 0.14)*0.5)*fade*part(0.0);
      }
    }
  }
  fx += fxFlakes(ro, rd, px, y0, y1, 3.5, 12.0, 2.2, 0.4, 0.5, 0.8, 0.8, 4.7, rose)*part(1.0);
  fx += fxAura(ro, rd, y0, y1, 2.4, gold)*0.4*part(2.0);
  fx += fxFlakes(ro, rd, px, y0, y1, 4.5, 10.0, 1.6, 0.45, 0.5, 0.9, 1.0, 9.7, vec3(1.0, 0.75, 0.6))*part(1.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 22.0, 0.03, 6.8, vec3(1.0, 0.9, 0.6))*part(2.0);
  return fx*uFx;
}
