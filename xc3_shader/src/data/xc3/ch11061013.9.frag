const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gTexMat[2];
    vec4 gWrkFl4[3];
    vec4 gWrkCol;
}U_Mate;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 0) uniform sampler2D s2;
layout(binding = 1) uniform sampler2D gTResidentTex09;
layout(binding = 2) uniform sampler2D s1;
layout(binding = 3) uniform sampler2D gTResidentTex03;
layout(binding = 4) uniform sampler2D s0;
layout(binding = 5) uniform sampler2D gTResidentTex05;
layout(binding = 6) uniform sampler2D gTResidentTex04;
layout(binding = 7) uniform sampler2D s3;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 4) in vec4 in_attr4;
layout(location = 5) in vec4 in_attr5;
layout(location = 6) in vec4 in_attr6;
layout(location = 7) in vec4 in_attr7;
layout(location = 8) in vec4 in_attr8;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
void main() {
    precise float temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise vec2 temp_4;
    precise float temp_5;
    precise float temp_6;
    precise vec2 temp_7;
    precise float temp_8;
    precise float temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise float temp_15;
    precise float temp_16;
    precise float temp_17;
    precise float temp_18;
    precise float temp_19;
    precise float temp_20;
    precise float temp_21;
    precise float temp_22;
    precise float temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    precise float temp_27;
    precise float temp_28;
    precise float temp_29;
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    precise float temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise float temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise float temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise float temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    precise float temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    precise float temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    precise float temp_66;
    precise float temp_67;
    precise float temp_68;
    precise float temp_69;
    precise float temp_70;
    precise float temp_71;
    precise float temp_72;
    precise float temp_73;
    precise float temp_74;
    precise float temp_75;
    precise float temp_76;
    precise float temp_77;
    precise float temp_78;
    precise float temp_79;
    precise float temp_80;
    precise float temp_81;
    precise float temp_82;
    precise float temp_83;
    precise float temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    precise float temp_110;
    precise float temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    precise float temp_115;
    precise float temp_116;
    precise float temp_117;
    precise float temp_118;
    precise float temp_119;
    precise float temp_120;
    precise float temp_121;
    precise float temp_122;
    precise float temp_123;
    precise float temp_124;
    precise float temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    precise float temp_129;
    precise float temp_130;
    precise float temp_131;
    precise float temp_132;
    precise float temp_133;
    precise vec2 temp_134;
    precise float temp_135;
    precise float temp_136;
    precise vec3 temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise vec3 temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise vec3 temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise vec3 temp_150;
    precise float temp_151;
    precise float temp_152;
    precise float temp_153;
    precise float temp_154;
    precise float temp_155;
    precise float temp_156;
    precise float temp_157;
    precise float temp_158;
    precise float temp_159;
    precise float temp_160;
    precise float temp_161;
    precise float temp_162;
    precise float temp_163;
    precise float temp_164;
    precise float temp_165;
    precise float temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    precise float temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    precise float temp_175;
    precise float temp_176;
    precise float temp_177;
    precise float temp_178;
    precise float temp_179;
    precise float temp_180;
    precise float temp_181;
    precise float temp_182;
    precise float temp_183;
    precise float temp_184;
    precise float temp_185;
    precise float temp_186;
    precise float temp_187;
    precise float temp_188;
    precise float temp_189;
    precise float temp_190;
    precise float temp_191;
    precise float temp_192;
    precise float temp_193;
    precise float temp_194;
    precise float temp_195;
    precise float temp_196;
    precise float temp_197;
    precise float temp_198;
    precise float temp_199;
    precise float temp_200;
    precise float temp_201;
    precise float temp_202;
    bool temp_203;
    precise float temp_204;
    bool temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    precise float temp_209;
    precise float temp_210;
    precise float temp_211;
    precise float temp_212;
    precise float temp_213;
    uint temp_214;
    precise float temp_215;
    precise float temp_216;
    precise float temp_217;
    precise float temp_218;
    precise float temp_219;
    precise float temp_220;
    precise float temp_221;
    precise float temp_222;
    precise float temp_223;
    int temp_224;
    precise float temp_225;
    precise float temp_226;
    precise float temp_227;
    precise float temp_228;
    precise float temp_229;
    precise float temp_230;
    precise float temp_231;
    precise float temp_232;
    precise float temp_233;
    precise float temp_234;
    precise float temp_235;
    precise float temp_236;
    precise float temp_237;
    precise float temp_238;
    precise float temp_239;
    precise float temp_240;
    precise float temp_241;
    temp_0 = in_attr4.x;
    temp_1 = in_attr4.y;
    temp_2 = in_attr4.z;
    temp_3 = in_attr4.w;
    temp_4 = texture(s2, vec2(temp_0, temp_1)).xy;
    temp_5 = temp_4.x;
    temp_6 = temp_4.y;
    temp_7 = texture(gTResidentTex09, vec2(temp_2, temp_3)).xy;
    temp_8 = temp_7.x;
    temp_9 = temp_7.y;
    temp_10 = in_attr1.x;
    temp_11 = in_attr1.y;
    temp_12 = in_attr1.z;
    temp_13 = in_attr0.x;
    temp_14 = in_attr0.z;
    temp_15 = temp_10 * temp_10;
    temp_16 = temp_13 * temp_13;
    temp_17 = fma(temp_11, temp_11, temp_15);
    temp_18 = fma(temp_12, temp_12, temp_17);
    temp_19 = inversesqrt(temp_18);
    temp_20 = temp_10 * temp_19;
    temp_21 = temp_11 * temp_19;
    temp_22 = temp_12 * temp_19;
    temp_23 = in_attr0.y;
    temp_24 = fma(temp_23, temp_23, temp_16);
    temp_25 = fma(temp_14, temp_14, temp_24);
    temp_26 = inversesqrt(temp_25);
    temp_27 = temp_13 * temp_26;
    temp_28 = temp_23 * temp_26;
    temp_29 = temp_14 * temp_26;
    temp_30 = fma(temp_5, 2., -1.0039216);
    temp_31 = fma(temp_6, 2., -1.0039216);
    temp_32 = fma(temp_8, 2., -1.0039216);
    temp_33 = fma(temp_9, 2., -1.0039216);
    temp_34 = temp_30 * temp_30;
    temp_35 = 0. + temp_30;
    temp_36 = temp_32 * temp_32;
    temp_37 = 0. + temp_31;
    temp_38 = fma(temp_31, temp_31, temp_34);
    temp_39 = 0. - temp_35;
    temp_40 = temp_32 * temp_39;
    temp_41 = fma(temp_33, temp_33, temp_36);
    temp_42 = 0. - temp_38;
    temp_43 = temp_42 + 1.;
    temp_44 = 0. - temp_37;
    temp_45 = fma(temp_33, temp_44, temp_40);
    temp_46 = sqrt(temp_43);
    temp_47 = 0. - temp_41;
    temp_48 = temp_47 + 1.;
    temp_49 = sqrt(temp_48);
    temp_50 = max(0., temp_46);
    temp_51 = max(0., temp_49);
    temp_52 = temp_50 + 1.;
    temp_53 = 0. - temp_52;
    temp_54 = temp_32 * temp_53;
    temp_55 = fma(temp_51, temp_52, temp_45);
    temp_56 = 0. - temp_52;
    temp_57 = temp_33 * temp_56;
    temp_58 = 0. - temp_54;
    temp_59 = fma(temp_35, temp_55, temp_58);
    temp_60 = 0. - temp_57;
    temp_61 = fma(temp_37, temp_55, temp_60);
    temp_62 = temp_51 * temp_52;
    temp_63 = in_attr2.x;
    temp_64 = temp_59 * temp_59;
    temp_65 = 0. - temp_62;
    temp_66 = fma(temp_52, temp_55, temp_65);
    temp_67 = in_attr2.y;
    temp_68 = fma(temp_61, temp_61, temp_64);
    temp_69 = in_attr2.z;
    temp_70 = fma(temp_66, temp_66, temp_68);
    temp_71 = inversesqrt(temp_70);
    temp_72 = temp_63 * temp_63;
    temp_73 = 0. - temp_30;
    temp_74 = fma(temp_59, temp_71, temp_73);
    temp_75 = 0. - temp_31;
    temp_76 = fma(temp_61, temp_71, temp_75);
    temp_77 = 0. - temp_50;
    temp_78 = fma(temp_66, temp_71, temp_77);
    temp_79 = in_attr3.z;
    temp_80 = fma(temp_67, temp_67, temp_72);
    temp_81 = fma(temp_74, U_Mate.gWrkFl4[2].x, temp_30);
    temp_82 = fma(temp_76, U_Mate.gWrkFl4[2].x, temp_31);
    temp_83 = in_attr3.x;
    temp_84 = fma(temp_78, U_Mate.gWrkFl4[2].x, temp_50);
    temp_85 = fma(temp_69, temp_69, temp_80);
    temp_86 = temp_81 * temp_81;
    temp_87 = inversesqrt(temp_85);
    temp_88 = fma(temp_82, temp_82, temp_86);
    temp_89 = fma(temp_84, temp_84, temp_88);
    temp_90 = temp_63 * temp_87;
    temp_91 = inversesqrt(temp_89);
    temp_92 = temp_67 * temp_87;
    temp_93 = in_attr3.y;
    temp_94 = temp_69 * temp_87;
    temp_95 = in_attr5.z;
    temp_96 = temp_81 * temp_91;
    temp_97 = temp_84 * temp_91;
    temp_98 = temp_82 * temp_91;
    temp_99 = temp_96 * temp_20;
    temp_100 = temp_96 * temp_21;
    temp_101 = temp_96 * temp_22;
    temp_102 = fma(temp_97, temp_27, temp_99);
    temp_103 = fma(temp_97, temp_28, temp_100);
    temp_104 = fma(temp_97, temp_29, temp_101);
    temp_105 = temp_83 * temp_83;
    temp_106 = fma(temp_98, temp_90, temp_102);
    temp_107 = fma(temp_98, temp_92, temp_103);
    temp_108 = fma(temp_98, temp_94, temp_104);
    temp_109 = fma(temp_93, temp_93, temp_105);
    temp_110 = temp_106 * temp_106;
    temp_111 = fma(temp_79, temp_79, temp_109);
    temp_112 = inversesqrt(temp_111);
    temp_113 = fma(temp_107, temp_107, temp_110);
    temp_114 = fma(temp_108, temp_108, temp_113);
    temp_115 = inversesqrt(temp_114);
    temp_116 = temp_83 * temp_112;
    temp_117 = in_attr5.w;
    temp_118 = temp_93 * temp_112;
    temp_119 = temp_79 * temp_112;
    temp_120 = in_attr5.x;
    temp_121 = temp_106 * temp_115;
    temp_122 = temp_107 * temp_115;
    temp_123 = temp_108 * temp_115;
    temp_124 = temp_121 * temp_116;
    temp_125 = fma(temp_122, temp_118, temp_124);
    temp_126 = fma(temp_123, temp_119, temp_125);
    temp_127 = temp_121 * temp_126;
    temp_128 = temp_122 * temp_126;
    temp_129 = fma(temp_127, -2., temp_116);
    temp_130 = fma(temp_128, -2., temp_118);
    temp_131 = fma(temp_129, U_Mate.gWrkFl4[1].x, 0.5);
    temp_132 = fma(temp_130, U_Mate.gWrkFl4[1].y, 0.5);
    temp_133 = in_attr5.y;
    temp_134 = texture(s1, vec2(temp_0, temp_1)).xy;
    temp_135 = temp_134.x;
    temp_136 = temp_134.y;
    temp_137 = textureLod(gTResidentTex03, vec2(temp_131, temp_132), 0.).xyz;
    temp_138 = temp_137.x;
    temp_139 = temp_137.y;
    temp_140 = temp_137.z;
    temp_141 = texture(s0, vec2(temp_0, temp_1)).xyz;
    temp_142 = temp_141.x;
    temp_143 = temp_141.y;
    temp_144 = temp_141.z;
    temp_145 = texture(gTResidentTex05, vec2(temp_95, temp_117)).x;
    temp_146 = texture(gTResidentTex04, vec2(temp_120, temp_133)).xyz;
    temp_147 = temp_146.x;
    temp_148 = temp_146.y;
    temp_149 = temp_146.z;
    temp_150 = texture(s3, vec2(temp_0, temp_1)).xyz;
    temp_151 = temp_150.x;
    temp_152 = temp_150.y;
    temp_153 = temp_150.z;
    temp_154 = in_attr8.w;
    temp_155 = in_attr8.x;
    temp_156 = in_attr8.y;
    temp_157 = 1. / temp_154;
    temp_158 = temp_155 * temp_157;
    temp_159 = in_attr7.x;
    temp_160 = temp_156 * temp_157;
    temp_161 = in_attr7.w;
    temp_162 = 1. / temp_161;
    temp_163 = 0. - temp_158;
    temp_164 = fma(temp_162, temp_159, temp_163);
    temp_165 = temp_136 * U_Mate.gWrkFl4[1].z;
    temp_166 = fma(temp_138, temp_135, temp_142);
    temp_167 = in_attr7.y;
    temp_168 = fma(temp_139, temp_135, temp_143);
    temp_169 = in_attr6.y;
    temp_170 = fma(temp_140, temp_135, temp_144);
    temp_171 = 0. - temp_165;
    temp_172 = fma(temp_165, temp_145, temp_171);
    temp_173 = 0. - temp_168;
    temp_174 = temp_173 + temp_148;
    temp_175 = 0. - temp_160;
    temp_176 = fma(temp_162, temp_167, temp_175);
    temp_177 = fma(temp_172, U_Mate.gWrkFl4[1].w, temp_165);
    temp_178 = 0. - temp_166;
    temp_179 = temp_178 + temp_147;
    temp_180 = temp_169 + 0.004;
    temp_181 = clamp(temp_180, 0., 1.);
    temp_182 = temp_176 * 0.5;
    temp_183 = temp_164 * 0.5;
    temp_184 = fma(temp_121, 0.5, 0.5);
    temp_185 = fma(temp_122, 0.5, 0.5);
    temp_186 = fma(temp_123, 1000., 0.5);
    temp_187 = fma(temp_179, temp_177, temp_166);
    temp_188 = 0. - temp_170;
    temp_189 = temp_188 + temp_149;
    temp_190 = abs(temp_183);
    temp_191 = abs(temp_182);
    temp_192 = max(temp_190, temp_191);
    temp_193 = fma(temp_174, temp_177, temp_168);
    temp_194 = fma(temp_189, temp_177, temp_170);
    temp_195 = max(temp_192, 1.);
    temp_196 = in_attr7.z;
    temp_197 = 1. / temp_195;
    temp_198 = in_attr6.x;
    temp_199 = temp_182 * temp_197;
    temp_200 = temp_183 * temp_197;
    temp_201 = abs(temp_200);
    temp_202 = inversesqrt(temp_201);
    temp_203 = temp_199 >= 0.;
    temp_204 = temp_203 ? 1. : 0.;
    temp_205 = temp_200 >= 0.;
    temp_206 = temp_205 ? 1. : 0.;
    temp_207 = 1. / temp_202;
    temp_208 = temp_196 * 8.;
    temp_209 = temp_181 * 3.;
    temp_210 = floor(temp_208);
    temp_211 = temp_204 * 0.6666667;
    temp_212 = trunc(temp_209);
    temp_213 = max(temp_212, 0.);
    temp_214 = uint(temp_213);
    temp_215 = temp_187 * 0.01;
    temp_216 = fma(temp_206, 0.33333334, temp_211);
    temp_217 = abs(temp_199);
    temp_218 = inversesqrt(temp_217);
    temp_219 = fma(temp_193, 0.01, temp_215);
    temp_220 = 0. - temp_210;
    temp_221 = temp_208 + temp_220;
    temp_222 = 1. / temp_218;
    temp_223 = temp_210 * 0.003921569;
    temp_224 = int(temp_214) << 6;
    temp_225 = floor(temp_223);
    temp_226 = fma(temp_194, 0.01, temp_219);
    temp_227 = float(uint(temp_224));
    temp_228 = temp_216 + 0.01;
    temp_229 = 0. - temp_187;
    temp_230 = temp_229 + temp_226;
    temp_231 = 0. - temp_193;
    temp_232 = temp_231 + temp_226;
    temp_233 = 0. - temp_194;
    temp_234 = temp_233 + temp_226;
    temp_235 = 0. - temp_225;
    temp_236 = temp_223 + temp_235;
    temp_237 = fma(temp_230, U_Mate.gWrkFl4[2].w, temp_187);
    temp_238 = fma(temp_232, U_Mate.gWrkFl4[2].w, temp_193);
    temp_239 = fma(temp_234, U_Mate.gWrkFl4[2].w, temp_194);
    temp_240 = temp_225 * 0.003921569;
    temp_241 = temp_227 * 0.003921569;
    out_attr0.x = temp_237;
    out_attr0.y = temp_238;
    out_attr0.z = temp_239;
    out_attr0.w = temp_241;
    out_attr1.x = temp_153;
    out_attr1.y = temp_151;
    out_attr1.z = U_Mate.gWrkFl4[2].y;
    out_attr1.w = 0.008235293;
    out_attr2.x = temp_184;
    out_attr2.y = temp_185;
    out_attr2.z = temp_152;
    out_attr2.w = temp_186;
    out_attr3.x = temp_207;
    out_attr3.y = temp_222;
    out_attr3.z = 0.;
    out_attr3.w = temp_228;
    out_attr4.x = temp_221;
    out_attr4.y = temp_236;
    out_attr4.z = temp_240;
    out_attr4.w = temp_198;
    return;
}