// Hyperspace: the blade is a ship's jump. A thin white-blue blade with a hard bright edge, and
// round it the starfield is stretched into streaks rushing up the blade toward the tip, faster
// and longer the nearer they pass, inside a mottled blue tunnel that swirls round the blade.
// Ignition is the jump itself: the stars sit as points, then stretch into lines. A swing pushes
// the streaks longer and brighter with a flash; a clash drops out of hyperspace for a moment:
// the streaks snap back to stars in a white ring.
// @room 8
// @palette {"corona.rim_cool":[0.08,0.25,0.85],"corona.rim_hot":[0.45,0.75,1.0],"corona.inner":[0.85,0.95,1.0],"core.white":[0.96,0.98,1.0],"core.fringe_cool":[0.25,0.55,1.0],"core.fringe_hot":[0.7,0.88,1.0],"light.color":[0.35,0.6,1.0],"trail":[0.4,0.65,1.0]}

// how stretched the stars are: 0 points, 1 full streaks (the jump on ignition, a drop on a clash)
float hy_stretch(){
  float jump = smoothstep(0.08, 0.5, uAge);
  float drop = 1.0 - smoothstep(0.0, 0.6, uClash)*0.9;
  return jump*drop*(1.0 + 0.8*clamp(uSwing, 0.0, 1.0));
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  px = min(px, 0.06);
  float L = uLength, t = uTime, ax = abs(b.x);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 blue = vec3(0.3, 0.55, 1.0), ice = vec3(0.75, 0.88, 1.0), white = vec3(1.0);
  float on = smoothstep(-0.4, 0.4, b.y)*(1.0 - smoothstep(L - 0.3, L + 0.5, b.y));
  float tipD = length(vec2(b.x, max(b.y - L, 0.0)));
  // the blade: a thin white core with a hard blue edge and a soft glow
  float core = fillD(ax - 0.28, px)*on;
  float edge = halo(abs(ax - 0.42), 0.07)*on;
  vec3 light = white*core*1.2 + blue*edge*1.2 + blue*halo(tipD, 0.9)*0.55*smoothstep(-0.5, 0.5, b.y) + blue*halo(tipD, 2.4)*0.12*smoothstep(-0.5, 0.5, b.y);
  float S = hy_stretch();
  // the streaks: rows of stars at different distances, rushing up the blade
  for (int i = 0; i < 36; i++){
    float fi = float(i);
    float lane = (h11(fi*3.7) - 0.5)*2.0;                 // -1..1 across
    float x = sign(lane)*(0.9 + 6.8*h11(fi*1.3)*h11(fi*1.3 + 0.5));
    float near = 1.0 - min(abs(x)/7.5, 1.0);
    float speed = 18.0 + 30.0*near;
    float y = fract(t*speed/(L + 12.0) + h11(fi*9.1))*(L + 12.0) - 6.0;
    float len = (0.25 + (2.0 + 6.0*near)*S);
    vec2 q = b - vec2(x, y);
    float along = clamp(q.y, -len, 0.0);
    float d = length(vec2(q.x, q.y - along));
    float tail = (along + len)/len;                       // 0 at the tail, 1 at the head
    float w = 0.03 + 0.05*near;
    vec3 col = mix(blue, white, 0.4 + 0.6*tail);
    light += col*(halo(d, max(w, px))*1.8 + halo(d, 0.25)*0.2)*(0.35 + 0.65*tail)*(0.5 + 0.5*near)*(1.0 + 0.8*sw);
  }
  // the tunnel: mottled blue bands swirling round the blade, brighter in front
  for (int j = 0; j < 2; j++){
    float fj = float(j);
    vec2 h = helix(b.y, 3.2 + 1.2*fj, 0.7 - 0.3*fj, t*(0.45 + 0.2*fj) + fj*0.37);
    float d = abs(b.x - h.x);
    float mott = 0.5 + 0.7*vn(vec2(b.y*0.6 - t*8.0, fj*5.0 + b.x*0.4));
    light += mix(blue, ice, 0.3)*halo(d, 0.9)*mott*0.28*(0.4 + 0.6*smoothstep(-0.5, 0.5, h.y))*S;
  }
  float swirl = fbm3(vec2(atan(b.x, 6.0)*3.0 - t*0.8, b.y*0.15 - t*2.2));
  light += blue*swirl*swirl*0.22*exp(-ax/5.0)*smoothstep(-1.0, 2.0, b.y)*(1.0 - smoothstep(L, L + 4.0, b.y))*S;
  // a swing's flash: a bright pulse racing up the streaks
  float puls = halo(abs(b.y - fract(t*1.4)*(L + 6.0) + 3.0), 1.2)*exp(-ax/3.0)*sw;
  light += ice*puls*0.8;
  // a clash: the drop out of hyperspace, a white ring and the stars snapping back
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.55);
    float rr = abs(length((b - o)*vec2(1.0, 0.6)) - 1.0 - 7.5*(1.0 - cl));
    light += white*halo(rr, 0.12 + 0.3*(1.0 - cl))*cl*1.6 + white*halo(length(b - o), 2.5)*pow(cl, 3.0)*1.5;
  }
  float fade = (1.0 - smoothstep(8.5, 10.0, ax))*(1.0 - smoothstep(L + 1.0, L + 4.0, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*fade*1.15))*1.05 + g*0.06;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.5; }
