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
    vec4 gWrkCol[2];
}U_Mate;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 0) uniform sampler2D s2;
layout(binding = 1) uniform sampler2D s0;
layout(binding = 2) uniform sampler2D s1;
layout(binding = 3) uniform sampler2D gTResidentTex05;
layout(binding = 4) uniform sampler2D gTResidentTex04;
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
    precise float temp_4;
    precise float temp_5;
    precise vec2 temp_6;
    precise float temp_7;
    precise float temp_8;
    precise vec3 temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise vec3 temp_15;
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
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise float temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise float temp_150;
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
    bool temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    bool temp_171;
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
    uint temp_189;
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
    precise float temp_203;
    precise float temp_204;
    int temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    precise float temp_209;
    precise float temp_210;
    precise float temp_211;
    precise float temp_212;
    precise float temp_213;
    precise float temp_214;
    temp_0 = in_attr4.x;
    temp_1 = in_attr4.y;
    temp_2 = in_attr5.x;
    temp_3 = in_attr5.y;
    temp_4 = in_attr4.w;
    temp_5 = in_attr4.z;
    temp_6 = texture(s2, vec2(temp_0, temp_1)).xy;
    temp_7 = temp_6.x;
    temp_8 = temp_6.y;
    temp_9 = texture(s0, vec2(temp_0, temp_1)).xyz;
    temp_10 = temp_9.x;
    temp_11 = temp_9.y;
    temp_12 = temp_9.z;
    temp_13 = texture(s1, vec2(temp_0, temp_1)).x;
    temp_14 = texture(gTResidentTex05, vec2(temp_2, temp_3)).x;
    temp_15 = texture(gTResidentTex04, vec2(temp_5, temp_4)).xyz;
    temp_16 = temp_15.x;
    temp_17 = temp_15.y;
    temp_18 = temp_15.z;
    temp_19 = in_attr1.x;
    temp_20 = in_attr0.x;
    temp_21 = in_attr1.y;
    temp_22 = in_attr0.y;
    temp_23 = in_attr1.z;
    temp_24 = in_attr0.z;
    temp_25 = in_attr2.y;
    temp_26 = in_attr2.x;
    temp_27 = temp_19 * temp_19;
    temp_28 = temp_20 * temp_20;
    temp_29 = fma(temp_21, temp_21, temp_27);
    temp_30 = fma(temp_22, temp_22, temp_28);
    temp_31 = fma(temp_23, temp_23, temp_29);
    temp_32 = fma(temp_24, temp_24, temp_30);
    temp_33 = in_attr2.z;
    temp_34 = inversesqrt(temp_31);
    temp_35 = inversesqrt(temp_32);
    temp_36 = temp_19 * temp_34;
    temp_37 = temp_21 * temp_34;
    temp_38 = temp_23 * temp_34;
    temp_39 = temp_20 * temp_35;
    temp_40 = temp_22 * temp_35;
    temp_41 = temp_24 * temp_35;
    temp_42 = fma(temp_7, 2., -1.0039216);
    temp_43 = temp_26 * temp_26;
    temp_44 = fma(temp_8, 2., -1.0039216);
    temp_45 = temp_13 * U_Mate.gWrkFl4[0].w;
    temp_46 = temp_42 * temp_36;
    temp_47 = in_attr3.y;
    temp_48 = fma(temp_25, temp_25, temp_43);
    temp_49 = temp_42 * temp_42;
    temp_50 = temp_42 * temp_37;
    temp_51 = temp_42 * temp_38;
    temp_52 = 0. - temp_12;
    temp_53 = temp_52 + U_Mate.gWrkCol[1].z;
    temp_54 = 0. - temp_45;
    temp_55 = fma(temp_45, temp_14, temp_54);
    temp_56 = fma(temp_33, temp_33, temp_48);
    temp_57 = fma(temp_44, temp_44, temp_49);
    temp_58 = inversesqrt(temp_56);
    temp_59 = 0. - temp_57;
    temp_60 = temp_59 + 1.;
    temp_61 = sqrt(temp_60);
    temp_62 = temp_26 * temp_58;
    temp_63 = temp_25 * temp_58;
    temp_64 = temp_33 * temp_58;
    temp_65 = max(0., temp_61);
    temp_66 = in_attr3.x;
    temp_67 = fma(temp_39, temp_65, temp_46);
    temp_68 = in_attr3.z;
    temp_69 = fma(temp_40, temp_65, temp_50);
    temp_70 = fma(temp_41, temp_65, temp_51);
    temp_71 = 0. - temp_10;
    temp_72 = temp_71 + U_Mate.gWrkCol[1].x;
    temp_73 = fma(temp_44, temp_62, temp_67);
    temp_74 = fma(temp_44, temp_63, temp_69);
    temp_75 = fma(temp_44, temp_64, temp_70);
    temp_76 = temp_73 * temp_73;
    temp_77 = fma(temp_74, temp_74, temp_76);
    temp_78 = temp_66 * temp_66;
    temp_79 = fma(temp_75, temp_75, temp_77);
    temp_80 = inversesqrt(temp_79);
    temp_81 = fma(temp_47, temp_47, temp_78);
    temp_82 = fma(temp_68, temp_68, temp_81);
    temp_83 = inversesqrt(temp_82);
    temp_84 = temp_73 * temp_80;
    temp_85 = in_attr7.x;
    temp_86 = temp_66 * temp_83;
    temp_87 = temp_68 * temp_83;
    temp_88 = temp_47 * temp_83;
    temp_89 = temp_74 * temp_80;
    temp_90 = temp_75 * temp_80;
    temp_91 = 0. - temp_86;
    temp_92 = temp_84 * temp_91;
    temp_93 = U_Mate.gWrkFl4[0].z * 5.;
    temp_94 = 0. - temp_88;
    temp_95 = fma(temp_89, temp_94, temp_92);
    temp_96 = 0. - temp_87;
    temp_97 = fma(temp_90, temp_96, temp_95);
    temp_98 = abs(temp_97);
    temp_99 = 0. - temp_98;
    temp_100 = temp_99 + 1.;
    temp_101 = log2(temp_100);
    temp_102 = dFdy(temp_90);
    temp_103 = in_attr7.w;
    temp_104 = dFdy(temp_84);
    temp_105 = temp_93 * temp_101;
    temp_106 = dFdy(temp_89);
    temp_107 = in_attr8.w;
    temp_108 = exp2(temp_105);
    temp_109 = fma(temp_108, temp_72, temp_10);
    temp_110 = 1. / temp_103;
    temp_111 = 0. - temp_11;
    temp_112 = temp_111 + U_Mate.gWrkCol[1].y;
    temp_113 = fma(temp_108, temp_53, temp_12);
    temp_114 = in_attr8.y;
    temp_115 = 1. * temp_104;
    temp_116 = 1. / temp_107;
    temp_117 = fma(temp_108, temp_112, temp_11);
    temp_118 = in_attr8.x;
    temp_119 = 1. * temp_106;
    temp_120 = in_attr7.z;
    temp_121 = 1. * temp_102;
    temp_122 = in_attr7.y;
    temp_123 = temp_115 * temp_115;
    temp_124 = fma(temp_119, temp_119, temp_123);
    temp_125 = 0. - temp_109;
    temp_126 = temp_125 + temp_16;
    temp_127 = 0. - temp_117;
    temp_128 = temp_127 + temp_17;
    temp_129 = fma(temp_121, temp_121, temp_124);
    temp_130 = fma(temp_55, U_Mate.gWrkFl4[1].x, temp_45);
    temp_131 = 0. - temp_113;
    temp_132 = temp_131 + temp_18;
    temp_133 = fma(temp_126, temp_130, temp_109);
    temp_134 = fma(temp_128, temp_130, temp_117);
    temp_135 = temp_118 * temp_116;
    temp_136 = temp_114 * temp_116;
    temp_137 = fma(temp_132, temp_130, temp_113);
    temp_138 = temp_133 * 0.01;
    temp_139 = 0. - temp_135;
    temp_140 = fma(temp_110, temp_85, temp_139);
    temp_141 = temp_120 * 8.;
    temp_142 = 0. - temp_136;
    temp_143 = fma(temp_110, temp_122, temp_142);
    temp_144 = 0. - U_Mate.gWrkFl4[1].w;
    temp_145 = 1. + temp_144;
    temp_146 = fma(temp_134, 0.01, temp_138);
    temp_147 = temp_140 * 0.5;
    temp_148 = temp_143 * 0.5;
    temp_149 = fma(temp_137, 0.01, temp_146);
    temp_150 = abs(temp_147);
    temp_151 = abs(temp_148);
    temp_152 = max(temp_150, temp_151);
    temp_153 = dFdx(temp_84);
    temp_154 = fma(temp_84, 0.5, 0.5);
    temp_155 = max(temp_152, 1.);
    temp_156 = dFdx(temp_89);
    temp_157 = 1. / temp_155;
    temp_158 = temp_153 * temp_153;
    temp_159 = fma(temp_89, 0.5, 0.5);
    temp_160 = fma(temp_90, 1000., 0.5);
    temp_161 = fma(temp_156, temp_156, temp_158);
    temp_162 = temp_148 * temp_157;
    temp_163 = in_attr6.y;
    temp_164 = temp_147 * temp_157;
    temp_165 = in_attr6.x;
    temp_166 = dFdx(temp_90);
    temp_167 = temp_162 >= 0.;
    temp_168 = temp_167 ? 1. : 0.;
    temp_169 = abs(temp_162);
    temp_170 = inversesqrt(temp_169);
    temp_171 = temp_164 >= 0.;
    temp_172 = temp_171 ? 1. : 0.;
    temp_173 = abs(temp_164);
    temp_174 = inversesqrt(temp_173);
    temp_175 = fma(temp_166, temp_166, temp_161);
    temp_176 = temp_175 + temp_129;
    temp_177 = 1. / temp_170;
    temp_178 = temp_168 * 0.6666667;
    temp_179 = 1. / temp_174;
    temp_180 = temp_163 + 0.004;
    temp_181 = clamp(temp_180, 0., 1.);
    temp_182 = temp_176 * 0.5;
    temp_183 = fma(temp_172, 0.33333334, temp_178);
    temp_184 = temp_181 * 3.;
    temp_185 = floor(temp_141);
    temp_186 = min(temp_182, 0.18);
    temp_187 = trunc(temp_184);
    temp_188 = max(temp_187, 0.);
    temp_189 = uint(temp_188);
    temp_190 = 0. - temp_133;
    temp_191 = temp_190 + temp_149;
    temp_192 = temp_183 + 0.01;
    temp_193 = fma(temp_145, temp_145, temp_186);
    temp_194 = clamp(temp_193, 0., 1.);
    temp_195 = 0. - temp_134;
    temp_196 = temp_195 + temp_149;
    temp_197 = sqrt(temp_194);
    temp_198 = 0. - temp_137;
    temp_199 = temp_198 + temp_149;
    temp_200 = fma(temp_191, U_Mate.gWrkFl4[2].z, temp_133);
    temp_201 = 0. - temp_185;
    temp_202 = temp_141 + temp_201;
    temp_203 = temp_185 * 0.003921569;
    temp_204 = floor(temp_203);
    temp_205 = int(temp_189) << 6;
    temp_206 = fma(temp_196, U_Mate.gWrkFl4[2].z, temp_134);
    temp_207 = float(uint(temp_205));
    temp_208 = fma(temp_199, U_Mate.gWrkFl4[2].z, temp_137);
    temp_209 = 0. - temp_197;
    temp_210 = temp_209 + 1.;
    temp_211 = 0. - temp_204;
    temp_212 = temp_203 + temp_211;
    temp_213 = temp_204 * 0.003921569;
    temp_214 = temp_207 * 0.003921569;
    out_attr0.x = temp_200;
    out_attr0.y = temp_206;
    out_attr0.z = temp_208;
    out_attr0.w = temp_214;
    out_attr1.x = U_Mate.gWrkFl4[2].x;
    out_attr1.y = temp_210;
    out_attr1.z = U_Mate.gWrkFl4[1].y;
    out_attr1.w = 0.008235293;
    out_attr2.x = temp_154;
    out_attr2.y = temp_159;
    out_attr2.z = 1.;
    out_attr2.w = temp_160;
    out_attr3.x = temp_179;
    out_attr3.y = temp_177;
    out_attr3.z = 0.;
    out_attr3.w = temp_192;
    out_attr4.x = temp_202;
    out_attr4.y = temp_212;
    out_attr4.z = temp_213;
    out_attr4.w = temp_165;
    return;
}
