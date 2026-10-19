// Deflector Shield: a starship's deflector field wrapped round the blade. A thin shell of shield
// cells shimmers round it, each cell slowly flickering; a swing brightens it and a clash flares
// the whole shell as if a bolt struck it.
// @part Shield
// @room 8
// @palette {"corona.rim_cool":[0.0,0.25,0.45],"corona.rim_hot":[0.25,0.85,1.0],"corona.inner":[0.75,0.97,1.0],"core.white":[0.92,1.0,1.0],"core.fringe_cool":[0.2,0.8,1.0],"core.fringe_hot":[0.75,0.97,1.0],"light.color":[0.25,0.85,1.0],"trail":[0.25,0.85,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.4, 0.9, 1.0);
  float s, tt;
  float d = raySeg(ro, rd, vec3(0.0, y0, 0.0), vec3(0.0, y1, 0.0), s);
  if (d < 3.0){
    float y = mix(y0, y1, s);
    float dd = rayPt(ro, rd, vec3(0.0, y, 0.0), tt);
    vec3 p = ro + rd*tt;
    vec2 cell;
    float e = voro(vec2(p.x*1.6, y*0.9), cell);
    float inside = 1.0 - smoothstep(1.6, 1.85, dd);
    float flick = 0.5 + 0.5*sin(t*1.3 + h21(cell)*6.2832);
    float hit = max(cl, 0.35*sw);
    float ends = smoothstep(0.0, 0.06, s)*(1.0 - smoothstep(0.94, 1.0, s));
    fx += c*(halo(abs(dd - 1.8), 0.08)*0.8 + halo(abs(dd - 1.8), 0.4)*0.2)*(1.0 + 1.5*hit)*ends;
    fx += c*halo(e, 0.04)*inside*(0.15 + 0.25*flick + 1.4*hit)*ends;
  }
  return fx*uFx*part(0.0);
}
