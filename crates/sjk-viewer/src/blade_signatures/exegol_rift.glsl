// Exegol Rift: a tear in space over the Sith world Exegol. The blade is a dark rift with a shadowed
// heart like Sol's Void, a far starfield glinting inside it; shadow copies trail every swing and
// cold lightning flickers now and then at its edge. A clash flashes the rift open.
// @part Rift lightning
// @room 8
// @palette {"corona.rim_cool":[0.08,0.1,0.35],"corona.rim_hot":[0.5,0.65,1.0],"corona.inner":[0.06,0.06,0.18],"core.white":[0.35,0.4,0.7],"core.fringe_cool":[0.3,0.4,0.9],"core.fringe_hot":[0.65,0.75,1.0],"light.color":[0.45,0.55,1.0],"trail":[0.45,0.55,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec2 q = vec2(b.x*6.0, b.y*1.5);
  vec2 c = floor(q);
  float st = step(0.9, h21(c))*halo(length(fract(q) - 0.5), 0.08)*(0.5 + 0.5*sin(uTime*3.0 + h21(c)*20.0));
  return g*(0.85 + 0.5*clamp(uClash, 0.0, 1.0)) + vec3(0.8, 0.85, 1.0)*st*lvl(g)*1.5;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.55, 0.7, 1.0);
  fx += fxArcs(ro, rd, px, y0, y1, 3.0, 3.0, 1.2, 6.6, c);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 14.0, -0.04, 1.1, vec3(0.8, 0.85, 1.0));
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, c);
  return fx*uFx*part(0.0);
}
