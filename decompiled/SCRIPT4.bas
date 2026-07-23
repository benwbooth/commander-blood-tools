[0000]   BLOCK (exit -> @00C9)
[0004]     ENDIF
[0005]     SETCHAR slot 2 = "maledict"
[0011]     SETCHAR slot 3 = "maledict"
[001D]     SETCHAR slot 4 = "maledict"
[0029]     SETCHAR slot 5 = "maledict"
[0035]     SETCHAR slot 1 = "maledict"
[0041]     SETCHAR slot 6 = "present"
[004C]     rec_0CD0 |= 0x2
[0051]     rec_049A = 4262
[0056]     scrujo.talk = 2984
[005B]     rec_00AA = 2954
[0060]     rec_00F2 = 3494
[0065]     rec_0842 = 2870
[006A]     rec_09AA = 3614
[006F]     rec_03C2 = 3140
[0074]     rec_040A = 4262
[0079]     rec_04E2 = 4462
[007E]     rec_07B2 = 3278
[0083]     rec_088A = 65535
[0088]     rec_052A = 65535
[008D]     rec_0692 = 65535
[0092]     rec_05BA = 65535
[0097]     rec_14CA = 40
[009C]     rec_1212 = 40
[00A1]     rec_1272 = 40
[00A6]     rec_0332 = 65535
[00AB]     rec_12D2 = 65535
[00B0]     maledict = 1
[00B7]     compris = 1
[00BE]     vbio = 3
[00C5]     POKE [0x0001] = 0
  END
[00C9]   BLOCK (exit -> @0147)
[00CD]     AWAIT gameflag_274F
[00CE]     GUARD active_actor == Bronko.talk (related 40)
[00D3]     ENDIF
[00D4]     SAY "Me busy do onboard cooking . Me make tasty meals for Cap'n Bob ..."
[00F8]     SAY "Let him do his job , Commander . It's about time we had something edible to eat ..."
[0124]     SAY "Me not have time , Commander ..."
[013A]     SAY "..."  '[skip 1]
[0144]     END PRESENTATION Bronko.talk
  END
[0147]   BLOCK (exit -> @0485)
[014B]     AWAIT gameflag_274F
[014C]     GUARD active_actor == Scruter_Jo.talk (related 40)
[0151]     ENDIF
[0152]     SAY "Commander you go get BIONIUM in CYBERSPACE of SCRUTER JO..."  '[voice 19]
[016E]     IF-BLOCK (exit -> @0315)
[0171]       GUARD compris == 0
[0178]       ENDIF
[0179]       SAY "Me explain to you how get BIONIUM..."
[018F]       SAY "You find BIOXX . Bioxx be small energy creatures ..."
[01AB]       SAY "You touch BIOXX once with hand ."  '[voice 1]
[01C1]       SAY "Sounds like a piece of cake to me , Commander !!"
[01DF]       SAY "If you touch BIOXX twice , you CAPTURE BIOXX on tip of your finger ..."  '[voice 2]
[0205]       SAY "Catch him on the tip of your finger !!! Sounds easy as pie , Commander ..."
[022D]       SAY "Then you can carry BIOXX to cybernetic MANTAS"  '[voice 3]
[0245]       SAY "You place BIOXX in belly of Manta..."  '[voice 5]
[025B]       SAY "BIOXX stay stuck to MANTAS..."  '[voice 4]
[026D]       SAY "I'd love to see that ..."
[0281]       SAY "Mantas change BIOXX into BIONIUM..."  '[voice 6]
[0293]       SAY "More BIOXX you give to Mantas, more BIONIUM you get ..."  '[voice 5]
[02B1]       SAY "Yes !!! BIONIUM ... I can taste it already ..."
[02CD]       SAY "To come back from CYBERSPACE , you touch BLUE BOX ...."  '[voice 4]
[02EB]       SAY "You understand ?"  '[voice 6]
[02F9]       SAY "We understand perfectly , Mister SCRUTER JO... Right , Commander?"
    END
[0315]     SAY "YOU go , Commander ..."  '[voice 20]
[0327]     SAY "Ahh! Me feel better ..."  '[voice 21]
[0339]     IF-BLOCK (exit -> @03DF)
[033C]       GUARD vbio > 0
[0343]       ENDIF
[0344]       SAY "Good work ... You did succeed ..."  '[voice 2]
[035A]       SAY "You did get BIONIUM..."  '[voice 3]
[036A]       SAY "YES !!! Commander , remind me to tell you you're a champ ..."
[038C]       SAY "This BIONIUM is extraordinary . My clock frequency's through the roof ..."
[03AC]       SAY "I feel even smarter ... I can feel I'll be a great help to you , Commander ..."  '[skip 1]
[03D8]       compris = 1
    END
[03DF]     IF-BLOCK (exit -> @0468)
[03E2]       GUARD vbio == 0
[03E9]       ENDIF
[03EA]       SAY "Not good , friend ... You fail ..."  '[voice 4]
[0402]       SAY "Commander, you didn't understand the technique ..."
[0418]       SAY "I need BIONIUM, Commander . It makes me smarter ..."
[0434]       SAY "Ha! Ha! You need much much BIONIUM . Ha! Ha!..."  '[voice 4]
[0450]       SAY "Why don't you shut up , wiseguy !!!"
    END
[0468]     SAY "Bye bye , Commander . Me return to CYBERSPACE..."  '[voice 7, skip 1]
[0482]     END PRESENTATION Scruter_Jo.talk
  END
[0485]   BLOCK (exit -> @04C7)
[0489]     AWAIT gameflag_274F
[048A]     GUARD active_actor == receiver.talk (related 40)
[048F]     ENDIF
[0490]     SAY "zzzzzzzzzz shshshshs... crak ..."
[04A0]     SAY "It won't work, Commander. We're too far away ..."
[04BA]     SAY "Krouikk..."  '[skip 1]
[04C4]     END PRESENTATION receiver.talk
  END
[04C7]   BLOCK (exit -> @055C)
[04CB]     AWAIT gameflag_274F
[04CC]     GUARD active_actor == Beauregard.talk (related 40)
[04D1]     GUARD maledict == 1
[04D8]     ENDIF
[04D9]     SAY "AAAAAHHH. Commander , I'm cursed..."  '[voice 23]
[04EB]     SAY "AAAAAAAAA !!!"  '[voice 24]
[04F7]     SAY "OOOOOOOO !!!"  '[voice 25]
[0503]     SAY "The the CURSE ...."  '[voice 30]
[0513]     SAY "hhhhhhhhhhh"  '[voice 29]
[051D]     SAY "CURSE..."  '[voice 30]
[0527]     SAY "Gee , Commander . That doesn't sound like fun ... !!!"
[0545]     SAY "aaaa..."  '[voice 30]
[054F]     SAY "..."  '[skip 1]
[0559]     END PRESENTATION Beauregard.talk
  END
[055C]   BLOCK (exit -> @069D)
[0560]     AWAIT gameflag_274F
[0561]     GUARD active_actor == Beauregard.talk (related 40)
[0566]     GUARD (rec_104E & 0x2) == 0
[056C]     GUARD maledict == 0
[0573]     ENDIF
[0574]     SAY "Ahhhh... Commander ... I feel better now ..."  '[voice 1]
[058C]     SAY "I saw the flames of hell, Commander. Not a pretty sight.."  '[voice 2]
[05AA]     SAY "Honk has explained our problem, Commander. Looks like the mummy of BETAKAM IV threw a spell on us..."  '[voice 3]
[05D6]     SAY "Commander ... I'd like to find the planet PATAGOS and meet king BETAKAM IV..."  '[voice 4]
[05FA]     SAY "It's amazing, Commander... We have his mummy aboard and yet we can meet him as a living being..."  '[voice 5]
[0626]     SAY "That's a unique archeological experiment..."  '[voice 4]
[0638]     SAY "According to the ancient texts from PATAGOS, Tumul translates as Attrox..."  '[voice 5]
[0656]     SAY "So the thing to do is find the planet Attrox, Commander..."  '[voice 1]
[0674]     SAY "Commander, Mister Beauregard is so amazingly intelligent..."
[068A]     SAY "See you soon, Commander..."  '[voice 1, skip 1]
[069A]     END PRESENTATION Beauregard.talk
  END
[069D]   BLOCK (exit -> @06FD)
[06A1]     AWAIT gameflag_274F
[06A2]     GUARD active_actor == Beauregard.talk (related 40)
[06A7]     GUARD (rec_104E & 0x2) != 0
[06AC]     GUARD maledict == 0
[06B3]     ENDIF
[06B4]     SAY "Ahhhh... Commander ... I feel better now ..."  '[voice 1]
[06CC]     SAY "I saw the flames of hell, Commander. Not a pretty sight.."  '[voice 2]
[06EA]     SAY "See you soon, Commander..."  '[voice 1, skip 1]
[06FA]     END PRESENTATION Beauregard.talk
  END
[06FD]   BLOCK (exit -> @08FA)
[0701]     AWAIT gameflag_252A
[0702]     GUARD rec_1148 == 3278
[0707]     GUARD active_actor == Fifi.talk (related 40)
[070C]     ENDIF
[070D]     SAY "Caa..."  '[voice 0]
[0717]     SAY "aake.."  '[voice 1]
[0721]     SAY "eat ..."  '[voice 1]
[072D]     IF-BLOCK (exit -> @07A8)
[0730]       GUARD rec_07D0 == 1
[0737]       ENDIF
[0738]       SAY "Me Fifi ... Hee Hee Hee ..."  '[voice 0]
[074E]       SAY "Heh heh... That guy's funny, Commander... I love his name ..."
[076C]       SAY "Fifi like gift"  '[voice 1]
[077A]       SAY "OOO! eat"  '[voice 2]
[0786]       SAY "After eat , Fifi sleep..."  '[voice 0]
[0798]       SAY "Hee hee hee ..."  '[voice 2]
    END
[07A8]     SAY "Me go . You bad vibrations ..."  '[voice 0]
[07BE]     SAY "Me scared ..."  '[voice 2]
[07CC]     SAY "Commander, maybe we could use the Mind Scrambler..."
[07E4]     SAY "USE MIND SCRAMBLER word_65535 use refuse"
[07FA]     IF-BLOCK (exit -> @0802)
[07FD]       GUARD concept == "use"
[0800]       ENDIF
[0801]       CLEAR concept_alt
    END
[0802]     IF-BLOCK (exit -> @0888)
[0805]       GUARD concept == "refuse"
[0808]       ENDIF
[0809]       SAY "You're calling the shots, Commander..."
[081B]       SAY "Bye bye ... Me go phone female Tromp"  '[voice 0]
[0833]       SAY "Trompette live on planet Magnu... Heee! Hee ! Hee! Me like female Tromp ..."  '[voice 2]
[0857]       SAY "Magnu be at coordinates x453 y543..."  '[voice 1, skip 1]
[086B]       rec_0DA8 |= 0x2
[0870]       SAY "Bye bye..."  '[skip 4]
[087C]       POKE [0x06FE] = 0
[0880]       POKE [0x08FB] = 1
[0884]       CLEAR concept_alt
[0885]       END PRESENTATION Fifi.talk
    END
[0888]     SAY "Zorglu... Eta ouch..."  '[voice 5, skip 1]
[0896]     LOADSTR "brouil.hnm"
[08A3]     SAY "Haga...."  '[voice 3]
[08AD]     SAY "HATA HATA..."  '[voice 4]
[08B9]     SAY "HOGLO HOGLO..."
[08C5]     SAY "You can interrogate him, Commander. He's ready..."  '[skip 1]
[08DB]     rec_07E0 = 6
[08E0]     SAY "Bye bye, Commander... word_65535 bye_bye"  '[skip 2]
[08F2]     rec_07E0 = 1882
[08F7]     END PRESENTATION Fifi.talk
  END
[08FA]   GOTO @0A6B
[08FE]   AWAIT gameflag_252A
[08FF]   rec_1148 = 3278
[0904]   START PRESENTATION Fifi.talk (related 40)
[0909]   ENDIF
[090A]   SAY "GA... Me Fifi ... Hee Hee Hee..."  '[voice 3]
[0920]   SAY "Heh heh... That guy knocks me out, Commander..."
[0938]   SAY "Fifi like cake"  '[voice 2]
[0946]   SAY "After eat, Fifi sleep..."  '[voice 0]
[0956]   SAY "Hee hee hee hee ..."  '[voice 2]
[0968]   IF-BLOCK (exit -> @09EA)
[096B]     GUARD rec_1242 == 65535
[0970]     ENDIF
[0971]     SAY "Give him the picture of the ondoyant, Commander... That'll make him happy..."
[0991]     SAY "TELEPORT PICTURE TO FIFI word_65535 teleport refuse"
[09A9]     IF-BLOCK (exit -> @09C8)
[09AC]       GUARD concept == "teleport"
[09AF]       ENDIF
[09B0]       SAY "TELEPORTING PICTURE TO FIFI"  '[skip 2]
[09C0]       OP_CD CD 30 00 2E 12 9A 07
[09C7]       CLEAR concept_alt
    END
[09C8]     IF-BLOCK (exit -> @09EA)
[09CB]       GUARD concept == "refuse"
[09CE]       ENDIF
[09CF]       SAY "If that's the way you want it , Commander..."  '[skip 1]
[09E9]       CLEAR concept_alt
    END
  END
[09EA]   IF-BLOCK (exit -> @0A52)
[09ED]     GUARD rec_1242 == 1946
[09F2]     ENDIF
[09F3]     SAY "Oh nice picture. Nice female . Me like..."  '[voice 1]
[0A0B]     SAY "Kiss... Kiss... Nice... Nice..."  '[voice 2]
[0A1B]     SAY "Me like ... Me like ... Me thank you ..."  '[voice 3]
[0A37]     SAY "Fifi go run ..."  '[voice 1, skip 3]
[0A47]     POKE [0x0A6C] = 1
[0A4B]     POKE [0x08FB] = 0
[0A4F]     END PRESENTATION Fifi.talk
  END
[0A52]   SAY "Fifi go run ... Bye bye ..."  '[voice 1, skip 1]
[0A68]   END PRESENTATION Fifi.talk
[0A6B]   GOTO @0BE6
[0A6F]   AWAIT gameflag_252A
[0A70]   rec_1148 = 3278
[0A75]   START PRESENTATION Fifi.talk (related 40)
[0A7A]   ENDIF
[0A7B]   SAY "Me Fifi ... Hee hee hee ..."  '[voice 3]
[0A91]   SAY "Heh heh ... I can't get enough of him, Commander..."
[0AAD]   SAY "Hee hee hee hee ..."  '[voice 2]
[0ABF]   SAY "Fifi like ondoyant... Ohh nice ..."  '[voice 0]
[0AD3]   IF-BLOCK (exit -> @0BCB)
[0AD6]     GUARD maledict == 0
[0ADD]     ENDIF
[0ADE]     SAY "Me give you gift. Me did find nice hat..."  '[voice 1]
[0AF8]     SAY "Me give you nice hat..."  '[voice 2]
[0B0A]     SAY "TELEPORT HAT TO CRYOBOX word_65535 teleport"
[0B20]     IF-BLOCK (exit -> @0B3F)
[0B23]       GUARD concept == "teleport"
[0B26]       ENDIF
[0B27]       SAY "TELEPORTING HAT TO CRYOBOX"  '[skip 2]
[0B37]       OP_CD CD D4 07 CE 14 28 00
[0B3E]       CLEAR concept_alt
    END
[0B3F]     SAY "But Commander, that's EVISCERATOR's hat... Where did he find it?"
[0B5B]     SAY "Commander, EVISCERATOR must be near here ..."
[0B71]     SAY "Who gave you the hat, Mister Fifi?"
[0B87]     SAY "Me find on ground ... Nice hat ..."  '[voice 1]
[0B9F]     SAY "Fifi go walk ... bye friend ..."  '[voice 0]
[0BB5]     SAY "..."  '[skip 3]
[0BBF]     rec_07B2 = 4262
[0BC4]     POKE [0x0A6C] = 0
[0BC8]     END PRESENTATION Fifi.talk
  END
[0BCB]   SAY "Fifi go walk ... Bye bye friend ..."  '[voice 0, skip 1]
[0BE3]   END PRESENTATION Fifi.talk
[0BE6]   BLOCK (exit -> @0BFD)
[0BEA]     GUARD rec_14E2 == 2450
[0BEF]     ENDIF
[0BF0]     rec_07B2 = 3278
[0BF5]     POKE [0x0BFE] = 1
[0BF9]     POKE [0x0BE7] = 0
  END
[0BFD]   GOTO @0D2F
[0C01]   AWAIT gameflag_252A
[0C02]   rec_07B2 = 3278
[0C07]   rec_1148 = 3278
[0C0C]   START PRESENTATION Fifi.talk (related 40)
[0C11]   ENDIF
[0C12]   SAY "Me Fifi ... Hee hee hee ..."  '[voice 3]
[0C28]   SAY "Heh heh... Still the same crazy guy he always was, Commander..."
[0C46]   SAY "Hee hee hee..."  '[voice 2]
[0C54]   SAY "Fifi like ondoyant... Ohh nice ..."  '[voice 0]
[0C68]   SAY "Fifi want go planet Malus ..."
[0C7C]   SAY "Fifi have friends planet MALUS ... You take me ?"
[0C98]   SAY "Howzabout we teleport him , Commander. He's such a character ..."
[0CB6]   SAY "TELEPORT FIFI TO CRYOBOX word_65535 teleport refuse"
[0CCE]   IF-BLOCK (exit -> @0CEE)
[0CD1]     GUARD concept == "teleport"
[0CD4]     ENDIF
[0CD5]     SAY "TELEPORTING FIFI TO CRYOBOX"  '[skip 2]
[0CE5]     rec_07B2 = 65535
[0CEA]     CLEAR concept_alt
[0CEB]     END PRESENTATION Fifi.talk
  END
[0CEE]   IF-BLOCK (exit -> @0D2F)
[0CF1]     GUARD concept == "refuse"
[0CF4]     ENDIF
[0CF5]     SAY "Me not happy... CRY CRY..."
[0D07]     SAY "How can you be so heartless , Commander ?"
[0D21]     SAY "..."  '[skip 2]
[0D2B]     CLEAR concept_alt
[0D2C]     END PRESENTATION Fifi.talk
  END
[0D2F]   BLOCK (exit -> @0E4F)
[0D33]     AWAIT gameflag_274F
[0D34]     GUARD active_actor == Fifi.talk (related 40)
[0D39]     ENDIF
[0D3A]     SAY "Fifi happy ... Hee hee hee..."  '[voice 3]
[0D4E]     SAY "Heh heh ... he'll never change , Commander ..."
[0D68]     IF-BLOCK (exit -> @0DF2)
[0D6B]       GUARD NOT rec_1148 == 3878
[0D71]       ENDIF
[0D72]       SAY "Hee hee hee ..."  '[voice 2]
[0D82]       SAY "Fifi like ondoyant ... Ohh nice ..."  '[voice 0]
[0D98]       SAY "Planet MALUS be coordinates x654 y765 ..."  '[skip 1]
[0DAE]       rec_0F28 |= 0x2
[0DB3]       SAY "You leave Fifi on MALUS, friend ... Me have friends ..."
[0DD1]       SAY "HEE ! HEE ! HEE !"
[0DE5]       SAY "..."  '[skip 1]
[0DEF]       END PRESENTATION Fifi.talk
    END
[0DF2]     IF-BLOCK (exit -> @0E4F)
[0DF5]       GUARD rec_1148 == 3878
[0DFA]       ENDIF
[0DFB]       SAY "You can teleport him , Commander ... We've arrived ..."
[0E17]       SAY "TELEPORT FIFI TO PLANET MALUS word_65535 teleport"
[0E2F]       IF-BLOCK (exit -> @0E4F)
[0E32]         GUARD concept == "teleport"
[0E35]         ENDIF
[0E36]         SAY "TELEPORTING FIFI TO MALUS"  '[skip 3]
[0E46]         rec_07B2 = 3878
[0E4B]         END PRESENTATION Fifi.talk
[0E4E]         CLEAR concept_alt
      END
    END
  END
[0E4F]   BLOCK (exit -> @0F7C)
[0E53]     AWAIT gameflag_252A
[0E54]     GUARD rec_07B2 == 3878
[0E59]     GUARD rec_1148 == 3878
[0E5E]     GUARD active_actor == Fifi.talk (related 40)
[0E63]     ENDIF
[0E64]     SAY "Me happy ... Hee hee hee ..."  '[voice 3]
[0E7A]     SAY "Heh heh ... he'll never change , Commander..."
[0E92]     SAY "Hee hee hee ..."  '[voice 2]
[0EA2]     SAY "Fifi like ondoyant ... Ohh nice ..."  '[voice 0]
[0EB8]     SAY "You transport Fifi here ... Fifi very happy ..."
[0ED2]     IF-BLOCK (exit -> @0F4B)
[0ED5]       GUARD (rec_0DE4 & 0x2) == 0
[0EDB]       ENDIF
[0EDC]       SAY "Fifi tell you big secret ... You think ondoyant not exist ..."
[0EFC]       SAY "You go Planet ONDOYA... Hee hee hee ... You have surprise..."
[0F1A]       SAY "What a joker, Commander..."
[0F2A]       SAY "Planet Ondoya be coordinates x432 y765 ... Hee hee hee..."  '[skip 1]
[0F46]       rec_0DE4 |= 0x2
    END
[0F4B]     SAY "Hee hee hee ... You nice ... Fifi like you ..."
[0F69]     SAY "Bye bye friend ..."  '[skip 1]
[0F79]     END PRESENTATION Fifi.talk
  END
[0F7C]   BLOCK (exit -> @1080)
[0F80]     AWAIT gameflag_252A
[0F81]     GUARD rec_1148 == 3554
[0F86]     GUARD active_actor == ondoyant.talk (related 40)
[0F8B]     ENDIF
[0F8C]     SAY "..."  '[voice 0]
[0F96]     SAY "..."  '[voice 1]
[0FA0]     SAY "Wowww!!! That is beautiful ... Aaaahhh..."
[0FB4]     SAY "Good to see you at last , my Commander..."  '[voice 2]
[0FCE]     SAY "I've been waiting billions of years for you ..."  '[voice 3]
[0FE8]     SAY "Commander ... This makes a nice change from TINA BURNER..."
[1004]     SAY "Take me with you ..."  '[voice 4]
[1016]     SAY "Commander... ondoyants exist after all ... I knew it ..."
[1032]     SAY "Let's teleport her and fast ..."
[1046]     SAY "TELEPORT ondoyant TO ARK... word_65535 teleport"
[105C]     IF-BLOCK (exit -> @1080)
[105F]       GUARD concept == "teleport"
[1062]       ENDIF
[1063]       SAY "TELEPORTING ondoyant VERY GENTLY TO ARK"  '[skip 3]
[1077]       rec_0572 = 65535
[107C]       CLEAR concept_alt
[107D]       END PRESENTATION ondoyant.talk
    END
  END
[1080]   BLOCK (exit -> @110C)
[1084]     AWAIT gameflag_274F
[1085]     GUARD active_actor == ondoyant.talk (related 40)
[108A]     ENDIF
[108B]     SAY "..."  '[voice 0]
[1095]     SAY "..."  '[voice 1]
[109F]     SAY "Woww!!! That is beautiful... Aaaahhh..."
[10B1]     SAY "Oh my Commander..."  '[voice 2]
[10BF]     SAY "I've been waiting billions of years for you ..."  '[voice 3]
[10D9]     SAY "Commander ... This makes a nice change from TINA BURNER..."
[10F5]     SAY "..."  '[voice 4]
[10FF]     SAY "..."  '[voice 5, skip 1]
[1109]     END PRESENTATION ondoyant.talk
  END
[110C]   BLOCK (exit -> @12F1)
[1110]     AWAIT gameflag_252A
[1111]     GUARD rec_1148 == 3494
[1116]     GUARD active_actor == Maziok.talk (related 40)
[111B]     ENDIF
[111C]     SAY "Ga ..."  '[voice 3]
[1128]     SAY "Hey, you know what he reminds me of? A ball of Tromp hair . Heh heh..."
[1150]     SAY "GA..."  '[voice 3]
[115A]     SAY "Ga ga ? word_65535 ga hello planet laugh joke"  '[voice 2]
[1176]     IF-BLOCK (exit -> @1199)
[1179]       GUARD concept == "ga"
[117C]       ENDIF
[117D]       SAY "ga ga ga ga ga ..."  '[skip 3]
[1191]       POKE [0x110D] = 0
[1195]       CLEAR concept_alt
[1196]       GOTO @136E
    END
[1199]     IF-BLOCK (exit -> @11A2)
[119C]       GUARD NOT concept == "ga"
[11A0]       ENDIF
[11A1]       CLEAR concept_alt
    END
[11A2]     SAY "GA ..."  '[voice 6]
[11AE]     SAY "Commander , I don't think he has the power of speech ... And get a load of that trunk..."
[11DC]     SAY "Looks like a Tromp-haired rat ! Ha! Ha! Ha !Ha! Ha!"
[11FA]     SAY "What makes you think MAZIOK can't speak ?"  '[voice 5]
[1212]     SAY "You have no respect for custom ..."  '[voice 4]
[1228]     SAY "When someone says GA , you have to answer GA . That's the tradition..."  '[voice 7]
[124C]     SAY "For generations I have said GA when soemone has said GA to me ..."  '[voice 4]
[1270]     SAY "You young people have no respect for anything ..."  '[voice 3]
[128A]     SAY "When I was young , we always said GA..."  '[voice 3]
[12A4]     SAY "You're weird , stranger. You look like you have the evil eye ..."  '[voice 7]
[12C6]     SAY "Leave me ... I have things to do ..."  '[voice 8]
[12E0]     SAY "..."  '[skip 2]
[12EA]     POKE [0x110D] = 0
[12EE]     END PRESENTATION Maziok.talk
  END
[12F1]   BLOCK (exit -> @14B7)
[12F5]     AWAIT gameflag_252A
[12F6]     GUARD rec_1148 == 3494
[12FB]     GUARD active_actor == Maziok.talk (related 40)
[1300]     GUARD maledict == 1
[1307]     ENDIF
[1308]     SAY "GA..."  '[voice 5]
[1312]     SAY "Ga... word_65535 ga hello weather_report work family fatherland"  '[voice 2]
[132C]     IF-BLOCK (exit -> @1348)
[132F]       GUARD concept == "ga"
[1332]       ENDIF
[1333]       SAY "You are most polite , stranger..."  '[voice 4, skip 1]
[1347]       CLEAR concept_alt
    END
[1348]     IF-BLOCK (exit -> @136E)
[134B]       GUARD NOT concept == "ga"
[134F]       ENDIF
[1350]       SAY "You are so rude ... Bye bye , stranger..."  '[voice 3, skip 2]
[136A]       END PRESENTATION Maziok.talk
[136D]       CLEAR concept_alt
    END
[136E]     SAY "Smart thinking , Commander ..."
[1380]     SAY "You're weird , stranger. You look like you have the evil eye ..."  '[voice 7]
[13A2]     SAY "You're weird ... You look like ... you're under a spell ..."  '[voice 5]
[13C2]     SAY "Or a curse ... Ha!!! Mmmm ..."  '[voice 4]
[13D8]     SAY "We've got a CURSE . I knew it , Commander..."
[13F4]     SAY "..."  '[voice 7]
[13FE]     IF-BLOCK (exit -> @145C)
[1401]       GUARD zen == 1
[1408]       ENDIF
[1409]       SAY "You need to get to CRAZYSTONE... X432 Y456 . You'll find a remedy there ..."  '[voice 4]
[142F]       SAY "Commander, I'm not feeling so good ..."
[1445]       SAY "Take care , stranger ..."  '[voice 4, skip 1]
[1457]       rec_0E20 |= 0x2
    END
[145C]     IF-BLOCK (exit -> @1492)
[145F]       GUARD (rec_0E20 & 0x2) != 0
[1464]       ENDIF
[1465]       SAY "Well , I have things to do ... Bye bye ... word_65535 bye_bye"  '[voice 8, skip 3]
[1487]       POKE [0x12F2] = 0
[148B]       POKE [0x14B8] = 1
[148F]       END PRESENTATION Maziok.talk
    END
[1492]     SAY "Well , I have things to do ... Bye bye .. word_65535 bye_bye"  '[voice 8, skip 1]
[14B4]     END PRESENTATION Maziok.talk
  END
[14B7]   GOTO @15AB
[14BB]   AWAIT gameflag_252A
[14BC]   rec_1148 = 3494
[14C1]   START PRESENTATION Maziok.talk (related 40)
[14C6]   maledict = 1
[14CD]   ENDIF
[14CE]   SAY "Ga... word_65535 ga hello weather_report work family fatherland"  '[voice 2]
[14E8]   IF-BLOCK (exit -> @1504)
[14EB]     GUARD concept == "ga"
[14EE]     ENDIF
[14EF]     SAY "You are most polite , stranger..."  '[voice 4, skip 1]
[1503]     CLEAR concept_alt
  END
[1504]   IF-BLOCK (exit -> @152A)
[1507]     GUARD NOT concept == "ga"
[150B]     ENDIF
[150C]     SAY "You are so rude ... Bye bye , stranger..."  '[voice 3, skip 2]
[1526]     END PRESENTATION Maziok.talk
[1529]     CLEAR concept_alt
  END
[152A]   SAY "You're weird , stranger. You look like you have the evil eye ..."  '[voice 7]
[154C]   SAY "You're weird ... You look like ... you're under a spell ..."  '[voice 5]
[156C]   SAY "Or a curse ... Ha!!! Mmmm ..."  '[voice 4]
[1582]   SAY "We've got a CURSE . I knew it , Commander..."
[159E]   SAY "..."  '[voice 7, skip 1]
[15A8]   END PRESENTATION Maziok.talk
[15AB]   BLOCK (exit -> @1788)
[15AF]     AWAIT gameflag_252A
[15B0]     GUARD rec_1148 == 3494
[15B5]     GUARD active_actor == Maziok.talk (related 40)
[15BA]     GUARD maledict == 0
[15C1]     ENDIF
[15C2]     SAY "GA..."  '[voice 5]
[15CC]     SAY "Ga... word_65535 ga hello weather_report work family fatherland"  '[voice 2]
[15E6]     IF-BLOCK (exit -> @1602)
[15E9]       GUARD concept == "ga"
[15EC]       ENDIF
[15ED]       SAY "ga ga ga ga ga ..."  '[skip 1]
[1601]       CLEAR concept_alt
    END
[1602]     IF-BLOCK (exit -> @1628)
[1605]       GUARD NOT concept == "ga"
[1609]       ENDIF
[160A]       SAY "You are so rude ... Bye bye , stranger..."  '[voice 5, skip 2]
[1624]       END PRESENTATION Maziok.talk
[1627]       CLEAR concept_alt
    END
[1628]     SAY "What do you want to know ?"  '[voice 2, skip 1]
[163E]     rec_0120 = 1244
[1643]     SAY "Ekato... word_65535 ekato"  '[voice 2, skip 1]
[1651]     ek = 1
[1658]     IF-BLOCK (exit -> @16EA)
[165B]       GUARD ek == 1
[1662]       ENDIF
[1663]       SAY "The planet Ekato is a very weird place ..."  '[voice 3]
[167D]       SAY "It's covered in jellyfish ..."  '[voice 3]
[168F]       SAY "I used to have some cousins there ..."  '[voice 1]
[16A7]       SAY "Ekato is at coordinates x567 y321 ..."  '[voice 3, skip 1]
[16BD]       rec_106C |= 0x2
[16C2]       SAY "Commander, if this keeps up , we'll have visited the entire galaxy ... Ha! Ha! Ha!"
    END
[16EA]     SAY "Sat is a very attractive desert planet... x456 y654... word_65535 tourism"  '[voice 1, skip 1]
[1708]     rec_0B1A |= 0x2
[170D]     SAY "Mister Maziok knows some real interesting stuff ... word_65535 tourism"
[1729]     IF-BLOCK (exit -> @1763)
[172C]       GUARD (rec_0B1A & 0x2) != 0
[1731]       GUARD ekato == est.value
[1738]       ENDIF
[1739]       SAY "Bye bye ... May the Great Yolk guide your trajectory ... word_65535 bye_bye"  '[skip 2]
[175B]       rec_00F2 = 4262
[1760]       END PRESENTATION Maziok.talk
    END
[1763]     SAY "Bye bye ... May the Great Yolk guide your trajectory ... word_65535 bye_bye"  '[skip 1]
[1785]     END PRESENTATION Maziok.talk
  END
[1788]   BLOCK (exit -> @1AB9)
[178C]     AWAIT gameflag_252A
[178D]     GUARD rec_1148 == 3110
[1792]     GUARD NOT rec_03C2 == 65535
[1798]     GUARD blood.talk == 3110
[179D]     GUARD active_actor == Bratakas.talk (related 40)
[17A2]     ENDIF
[17A3]     IF-BLOCK (exit -> @1828)
[17A6]       GUARD rec_0080 == 1
[17AD]       ENDIF
[17AE]       SAY "You seem to fear nothing ... Why have you come to my planet , stranger ?"  '[voice 0]
[17D6]       SAY "Have you any idea who you're talking to ? I am none other than Bratakas , terror of the planet VISTAR ..."  '[voice 2]
[180A]       SAY "My name strikes fear in all Tromps throughout the galaxy ..."  '[voice 1]
    END
[1828]     SAY "How can I help you , stranger ?"  '[voice 2, skip 1]
[1840]     rec_0090 = 1
[1845]     IF-BLOCK (exit -> @18A4)
[1848]       GUARD planete == 1
[184F]       ENDIF
[1850]       SAY "I know a few planets. let me see ... My memory's letting me down here ..."  '[voice 0]
[1878]       SAY "The planet ... I can't seem to recall ..."  '[voice 1]
[1892]       SAY "It'll come to me ..."  '[voice 2]
    END
[18A4]     IF-BLOCK (exit -> @192B)
[18A7]       GUARD decodeur == 1
[18AE]       GUARD NOT rec_12D2 == 74
[18B4]       ENDIF
[18B5]       SAY "Commander, what say we give him our old decoder ... He won't pick up very much , but it might make him happy ..."
[18ED]       rec_0090 = 1882
[18F2]       SAY "TELEPORT OLD DECODER TO PLANET VISTAR word_65535 teleport"
[190C]       IF-BLOCK (exit -> @192B)
[190F]         GUARD concept == "teleport"
[1912]         ENDIF
[1913]         SAY "TELEPORT DECODER TO BRATAKAS"  '[skip 2]
[1923]         OP_CD CD 30 00 BE 12 4A 00
[192A]         CLEAR concept_alt
      END
    END
[192B]     IF-BLOCK (exit -> @1A43)
[192E]       GUARD rec_12D2 == 74
[1933]       GUARD NOT rec_1242 == 40
[1939]       ENDIF
[193A]       SAY "Oh , what a nice decoder ... That's quite a gift , stranger ..."  '[voice 1]
[195E]       rec_0090 = 1882
[1963]       SAY "I wonder what I can offer you in exchange ... What would you say to a pretty picture ?"  '[voice 2]
[1991]       SAY "It represents the female of your dreams . What I see is a female Trump ..."  '[voice 0]
[19B9]       SAY "They're called ondoyants ... Dream creatures ..."  '[voice 1]
[19CF]       SAY "This is for you , stranger ..."  '[voice 0]
[19E5]       SAY "Don't look at it too often ... And don't show it to anybody ..."  '[voice 1]
[1A09]       SAY "TELEPORT PICTURE TO CRYOBOX word_65535 teleport"
[1A1F]       IF-BLOCK (exit -> @1A43)
[1A22]         GUARD concept == "teleport"
[1A25]         ENDIF
[1A26]         SAY "TELEPORT PICTURE TO CRYOBOX"  '[skip 3]
[1A36]         OP_CD CD 84 00 2E 12 28 00
[1A3D]         rec_0090 = 1
[1A42]         CLEAR concept_alt
      END
    END
[1A43]     IF-BLOCK (exit -> @1A62)
[1A46]       GUARD concept == "bye_bye"
[1A49]       GUARD decodeur == 0
[1A50]       ENDIF
[1A51]       SAY "Bye bye ..."  '[skip 1]
[1A5F]       END PRESENTATION Bratakas.talk
    END
[1A62]     IF-BLOCK (exit -> @1AB9)
[1A65]       GUARD concept == "bye_bye"
[1A68]       GUARD decodeur == 1
[1A6F]       ENDIF
[1A70]       SAY "Bye bye ... Leaving so soon ?"  '[voice 1]
[1A86]       SAY "Hold on ... There's someone who'd like to see you before you leave ..."  '[voice 2]
[1AAA]       SAY "..."  '[skip 1]
[1AB4]       OP_C1 C1 F0 14 44 0C
    END
  END
[1AB9]   BLOCK (exit -> @1AD2)
[1ABD]     GUARD rec_12D2 == 74
[1AC2]     GUARD rec_1242 == 40
[1AC7]     GUARD rec_03C2 == 65535
[1ACC]     ENDIF
[1ACD]     blood.talk = 4262
  END
[1AD2]   BLOCK (exit -> @1E59)
[1AD6]     AWAIT gameflag_252A
[1AD7]     GUARD rec_1148 == 3110
[1ADC]     GUARD active_actor == Hom.talk (related 40)
[1AE1]     ENDIF
[1AE2]     SAY "Welcome , stranger . My name is Hom ..."  '[voice 1, skip 1]
[1AFC]     rec_03F0 = 6
[1B01]     SAY "Make yourself comfortable ..."  '[voice 2]
[1B11]     IF-BLOCK (exit -> @1BEF)
[1B14]       GUARD rec_03E0 == 1
[1B1B]       ENDIF
[1B1C]       SAY "BUT COMMANDER , IT'S HOM... HE ALREADY EXISTED AT THIS TIME ..."
[1B3C]       SAY "You good , stranger . You did give decoder to BRATAKAS... Me LIKE you ..."  '[voice 2]
[1B62]       SAY "He didn't try his print stunt , Commander..."
[1B7A]       SAY "Probably didn't exist at this time ..."
[1B90]       SAY "YOU CLICK HARD ? FRIEND ... ME CATCH PRINT ... word_65535 click"  '[voice 4]
[1BB2]       IF-BLOCK (exit -> @1BCE)
[1BB5]         GUARD concept == "click"
[1BB8]         ENDIF
[1BB9]         SAY "Forget I spoke , Commander ..."
[1BCD]         CLEAR concept_alt
      END
[1BCE]       SAY "Ohh ! Nice print ..."  '[voice 5, skip 1]
[1BE0]       LOADSTR "scandoig.hnm"
    END
[1BEF]     IF-BLOCK (exit -> @1C84)
[1BF2]       GUARD rec_1242 == 40
[1BF7]       GUARD rec_12D2 == 74
[1BFC]       ENDIF
[1BFD]       SAY "Commander, I have an idea ..."
[1C11]       SAY "Why not give him our D.O.R.K. diploma ? The one we so brilliantly earned on Cyberock..."
[1C39]       SAY "Maybe that'll remind him of something ..."
[1C4F]       SAY "TELEPORT D.O.R.K. TO HOM word_65535 teleport"
[1C65]       IF-BLOCK (exit -> @1C84)
[1C68]         GUARD concept == "teleport"
[1C6B]         ENDIF
[1C6C]         SAY "TELEPORTING D.O.R.K. TO HOM..."  '[skip 2]
[1C7C]         OP_CD CD 30 00 5E 12 AA 03
[1C83]         CLEAR concept_alt
      END
    END
[1C84]     IF-BLOCK (exit -> @1E26)
[1C87]       GUARD rec_1272 == 938
[1C8C]       ENDIF
[1C8D]       SAY "But ... This be D.O.R.K... You with "GUILD OF MEMBERS" ..."  '[voice 1]
[1CAB]       SAY "But date all wrong ... This date be in future ... Not yet happen ..."  '[voice 2]
[1CD1]       SAY "You be mysterious , friend ..."  '[voice 3]
[1CE5]       SAY "You sure got him with that slick move , Commander..."
[1D01]       SAY "Me not understand ..."  '[voice 4]
[1D11]       SAY "We're from the FUTURE , Mister Hom!!!"
[1D27]       SAY "From the future !!!"  '[voice 5]
[1D37]       SAY "We have a big ship that travels through BLACK HOLES ... We're looking for the BIG BANG ..."
[1D63]       SAY "Crazy , huh ..."
[1D73]       SAY "Big Bang... Black holes ... Big ship ... You be crazy okay ..."  '[voice 6]
[1D95]       SAY "Me like that ... Big Bang... Black holes ..."  '[voice 7]
[1DAF]       SAY "Me want come with you ... Me have big tubular brain ..."  '[voice 8]
[1DCF]       SAY "Me have CDROM in head ... Me know universe ..."  '[voice 9, skip 1]
[1DEB]       rec_03F0 = 6
[1DF0]       SAY "TELEPORT HOM TO ARK word_65535 teleport"
[1E06]       IF-BLOCK (exit -> @1E26)
[1E09]         GUARD concept == "teleport"
[1E0C]         ENDIF
[1E0D]         SAY "TELEPORTING HOM TO CRYOBOX"  '[skip 3]
[1E1D]         rec_03C2 = 65535
[1E22]         CLEAR concept_alt
[1E23]         END PRESENTATION Hom.talk
      END
    END
[1E26]     SAY "What you want know , friend ? Me be OMNISCIENT... ME KNOW EVERYTHING ..."  '[voice 4, skip 1]
[1E4A]     rec_03F0 = 6
[1E4F]     IF-BLOCK (exit -> @1E59)
[1E52]       GUARD concept == "bye_bye"
[1E55]       ENDIF
[1E56]       END PRESENTATION Hom.talk
    END
  END
[1E59]   BLOCK (exit -> @1EA3)
[1E5D]     AWAIT gameflag_274F
[1E5E]     GUARD active_actor == Hom.talk (related 40)
[1E63]     ENDIF
[1E64]     SAY "Me HOM..."  '[voice 6]
[1E70]     SAY "Me be OMNISCIENT... ME KNOW EVERYTHING ..."  '[voice 1]
[1E86]     SAY "Bye bye ... Me sleep in star dust ..."  '[voice 2, skip 1]
[1EA0]     END PRESENTATION Hom.talk
  END
[1EA3]   BLOCK (exit -> @21AD)
[1EA7]     AWAIT gameflag_252A
[1EA8]     GUARD active_actor == Super_Zen.talk (related 40)
[1EAD]     GUARD NOT rec_122A == 40
[1EB3]     GUARD rec_1148 == 3614
[1EB8]     ENDIF
[1EB9]     SAY "..."
[1EC3]     SAY "..."
[1ECD]     SAY "..."
[1ED7]     SAY "Shhh , Commander . Don't disturb him ..."
[1EEF]     SAY "I sense the magic of the PATAGOS ... STRONG MAGIC ..."  '[voice 0]
[1F0D]     SAY "..."
[1F17]     SAY "..."
[1F21]     SAY "He's concentrating , Commander..."
[1F31]     SAY "Magic magic ... It ... It's the CURSE OF THE MUMMY OF BETAKAM IV OF THE PATAGOS !!!"  '[voice 1]
[1F5D]     SAY "IT'S IMPOSSIBLE ... HE ISN'T DEAD YET ... IT'S MAGIC ..."  '[voice 2]
[1F7B]     SAY "Commander, the poor guy doesn't know we brought the mummy from the future . Ha! Ha! Ha!"
[1FA5]     SAY "How can this be ?"  '[voice 3]
[1FB7]     SAY "Excuse me a moment ... I'm going to TELEPATH to BETAKAM to get to the bottom of this mystery ..."  '[voice 4]
[1FE7]     SAY "Be silent when I telepath ..."  '[voice 0]
[1FFB]     SAY "Calling BETAKAM IV on the neuronal channel ... Come in !"  '[voice 1]
[2019]     SAY "It's me , Super ZEN ... Something incredible's just happened ..."  '[voice 2]
[2037]     SAY "..."
[2041]     SAY "Commander, he's telling him the story telepathically ..."
[2059]     SAY "..."
[2063]     SAY "Ha! Ha! He thought your story was very funny ..."  '[voice 3]
[207F]     SAY "So ... How much have you got ? word_65535 nothing ship hope"  '[voice 4]
[20A1]     IF-BLOCK (exit -> @20A9)
[20A4]       GUARD concept == "nothing"
[20A7]       ENDIF
[20A8]       CLEAR concept_alt
    END
[20A9]     IF-BLOCK (exit -> @20B2)
[20AC]       GUARD NOT concept == "nothing"
[20B0]       ENDIF
[20B1]       CLEAR concept_alt
    END
[20B2]     SAY "I see ..."  '[voice 2]
[20C0]     SAY "Look ... I have a mission for you ..."  '[voice 3]
[20DA]     SAY "I want you to steal the portrait of the GREAT YOLK for me ... from the planet VISTA ..."  '[voice 4]
[2108]     SAY "The planet VISTA is at coordinates x453 y654 ..."  '[voice 2, skip 1]
[2122]     rec_0C5E |= 0x2
[2127]     SAY "How about that ! He wants us to steal the painting of the GREAT YOLK !"
[214F]     SAY "When you bring me the portrait , painted in the GREAT YOLK's own yellow blood , I'll remove your curse ..."  '[voice 3]
[2181]     SAY "See you soon , my friend ..."  '[voice 4]
[2197]     SAY "..."  '[skip 3]
[21A1]     POKE [0x1EA4] = 0
[21A5]     rec_09AA = 4262
[21AA]     END PRESENTATION Super_Zen.talk
  END
[21AD]   BLOCK (exit -> @21C0)
[21B1]     GUARD rec_122A == 40
[21B6]     ENDIF
[21B7]     rec_09AA = 3614
[21BC]     POKE [0x21AE] = 0
  END
[21C0]   BLOCK (exit -> @2230)
[21C4]     AWAIT gameflag_252A
[21C5]     GUARD active_actor == Super_Zen.talk (related 40)
[21CA]     GUARD rec_122A == 40
[21CF]     GUARD rec_1148 == 3614
[21D4]     ENDIF
[21D5]     SAY "..."
[21DF]     SAY "Well ... You are indeed talented ..."  '[voice 0]
[21F5]     SAY "Teleport him the painting , Commander... word_65535 teleport"
[220F]     IF-BLOCK (exit -> @2230)
[2212]       GUARD concept == "teleport"
[2215]       ENDIF
[2216]       SAY "TELEPORTING PAINTING TO PLANET CRAZYSTONE"  '[skip 2]
[2228]       OP_CD CD 30 00 16 12 92 09
[222F]       CLEAR concept_alt
    END
  END
[2230]   BLOCK (exit -> @2462)
[2234]     AWAIT gameflag_252A
[2235]     GUARD active_actor == Super_Zen.talk (related 40)
[223A]     GUARD rec_122A == 2450
[223F]     GUARD rec_1148 == 3614
[2244]     ENDIF
[2245]     SAY "NICE MOVE , Commander..."
[2255]     SAY "If you're ready , we'll begin the disenchantment ceremony .."  '[voice 5]
[2271]     SAY "..."
[227B]     SAY "..."
[2285]     SAY "Look , Commander ! He's concentrating ... I think ..."
[22A1]     SAY "ATA ATA HOGLO HULU..."  '[voice 9]
[22B1]     SAY "ATA ATA HOGLO HULU..."  '[voice 10]
[22C1]     SAY "HAM TOT ZAGLO HOLO HULU..."  '[skip 1]
[22D3]     LOADSTR "tourbi.hnm"
[22E0]     SAY "ATA ATA HOGLO HULU..."  '[voice 8]
[22F0]     SAY "ATA ATA HOGLO HULU..."  '[skip 1]
[2300]     LOADSTR "momi10.hnm"
[230D]     SAY "HER TOT ZAGLO HOLO HULU..."  '[skip 1]
[231F]     LOADSTR "star2.hnm"
[232B]     SAY "Hey , Commander ! The curse has been lifted !"
[2347]     SAY "I'll bet Mister Beauregard is happy about it ..."
[2361]     SAY "Shhh ... It's not finished ..."  '[voice 6]
[2375]     SAY "The curse of Betakam IV is the sneakiest curse I know ..."  '[voice 7]
[2395]     SAY "Betakam would like to meet you ..."  '[voice 2]
[23AB]     SAY "He can't figure out how you were cursed by his mummy , since he's still alive ..."  '[voice 0]
[23D5]     SAY "You'll find him on the planet Attrox, coordinates x342 y765..."  '[voice 1, skip 1]
[23F1]     rec_104E |= 0x2
[23F6]     SETCHAR slot 1 = "present"
[2401]     SETCHAR slot 2 = "present"
[240C]     SETCHAR slot 3 = "present"
[2417]     SETCHAR slot 4 = "present"
[2422]     SETCHAR slot 5 = "present"
[242D]     SAY "Commander... This is just great !"
[2441]     SAY "Bye bye ..."  '[voice 0, skip 4]
[244F]     maledict = 0
[2456]     POKE [0x2231] = 0
[245A]     rec_09AA = 4262
[245F]     END PRESENTATION Super_Zen.talk
  END
[2462]   BLOCK (exit -> @247A)
[2466]     GUARD rec_122A == 2450
[246B]     GUARD rec_14E2 == 65535
[2470]     ENDIF
[2471]     rec_09AA = 3614
[2476]     POKE [0x2463] = 0
  END
[247A]   BLOCK (exit -> @272E)
[247E]     AWAIT gameflag_252A
[247F]     GUARD active_actor == Super_Zen.talk (related 40)
[2484]     GUARD rec_122A == 2450
[2489]     GUARD rec_1148 == 3614
[248E]     ENDIF
[248F]     SAY "..."
[2499]     SAY "..."
[24A3]     SAY "..."
[24AD]     SAY "..."
[24B7]     SAY "..."
[24C1]     SAY "Commander, I think he's meditating ..."
[24D5]     IF-BLOCK (exit -> @2577)
[24D8]       GUARD rec_14E2 == 65535
[24DD]       ENDIF
[24DE]       SAY "Welcome friend ... How can I help you ?"  '[voice 0]
[24F8]       SAY "Why don't we give him Eviscerator's hat , Commander ... He might give us some information ..."
[2522]       SAY "TELEPORT HAT TO SUPER ZEN word_65535 teleport refuse"
[253C]       IF-BLOCK (exit -> @255D)
[253F]         GUARD concept == "teleport"
[2542]         ENDIF
[2543]         SAY "TELEPORTING HAT TO SUPER ZEN"  '[skip 2]
[2555]         OP_CD CD 30 00 CE 14 92 09
[255C]         CLEAR concept_alt
      END
[255D]       IF-BLOCK (exit -> @2577)
[2560]         GUARD concept == "refuse"
[2563]         ENDIF
[2564]         SAY "It's your decision , Commander..."
[2576]         CLEAR concept_alt
      END
    END
[2577]     IF-BLOCK (exit -> @270F)
[257A]       GUARD rec_14E2 == 2450
[257F]       ENDIF
[2580]       SAY "OOOh... A hat ..."  '[voice 0]
[2590]       SAY "Uuh ! Bad vibrations... I sense fury and noisy ..."  '[voice 1]
[25AC]       SAY "I sense pain , blood and tears ..."  '[voice 2]
[25C4]       SAY "I see ... Ha !!! A planet ... Big ships ..."  '[voice 3]
[25E2]       SAY "The planet is called ..."  '[voice 0]
[25F4]       SAY "He's gonna say it , Commander ... I can feel it ..."
[2614]       SAY "The planet is called ..."  '[voice 0]
[2626]       SAY "Okay ... Cut the suspense !!!"
[263A]       SAY "MASTA ..."  '[voice 0]
[2646]       SAY "MASTACHOK... I knew it , Commander ..."
[265C]       SAY "Not Mastachok ! MASTA . Coordinates x564 y098..."  '[voice 0, skip 1]
[2674]       rec_0B8C |= 0x2
[2679]       SAY "YES !!! Got it ... MASTA ... What kind of a name is that for a planet ..."
[26A5]       SAY "Time to make tracks , Commander ..."
[26BB]       SAY "You did good , Mister Zen . Thanks a zill ..."
[26D9]       SAY "So long , my friends ... I must travel for some time now ..."  '[voice 4]
[26FD]       SAY "..."  '[skip 2]
[2707]       rec_09AA = 4262
[270C]       END PRESENTATION Super_Zen.talk
    END
[270F]     SAY "We'll come back later ..."
[2721]     SAY "..."  '[skip 1]
[272B]     END PRESENTATION Super_Zen.talk
  END
[272E]   BLOCK (exit -> @2C4C)
[2732]     AWAIT gameflag_252A
[2733]     GUARD rec_1148 == 4172
[2738]     GUARD active_actor == Betakam.talk (related 40)
[273D]     ENDIF
[273E]     SAY "..."  '[voice 1]
[2748]     SAY "..."  '[voice 1]
[2752]     SAY "..."  '[voice 1]
[275C]     SAY "Welcome to the planet Attrox , stranger ..."  '[voice 0]
[2774]     SAY "..."  '[voice 1]
[277E]     SAY "..."  '[voice 1]
[2788]     SAY "..."  '[voice 1]
[2792]     SAY "I am Betakam IV , king of the PATAGOS ..."
[27AE]     SAY "You see the sun up in the sky ? It is named GLADIS ..."
[27D2]     SAY "..."  '[voice 1]
[27DC]     SAY "..."  '[voice 1]
[27E6]     SAY "..."  '[voice 1]
[27F0]     SAY "Its heat warms the PATAGOS ..."
[2804]     SAY "Super Zen spoke to me of you ! Some story about a CURSE !!"
[2828]     SAY "Commander, this is rich ... We have that guy's mummy in the Ark !"
[284C]     SAY "Commander , I'd like to wake up Mister Beauregard . He's the only one who can explain some of this ..."
[287E]     SAY "Is that okay with you , Commander ? word_65535 accept refuse"
[289E]     IF-BLOCK (exit -> @28A6)
[28A1]       GUARD concept == "accept"
[28A4]       ENDIF
[28A5]       CLEAR concept_alt
    END
[28A6]     IF-BLOCK (exit -> @28E3)
[28A9]       GUARD concept == "refuse"
[28AC]       ENDIF
[28AD]       SAY "You're the bossman , Commander..."
[28BF]       SAY "But I have a feeling we're out of our depth here ..."  '[skip 2]
[28DF]       CLEAR concept_alt
[28E0]       END PRESENTATION Betakam.talk
    END
[28E3]     SAY "Ok , Commander... Mister Beauregard , wakey wakey !"
[28FD]     SAY "Yes ... I'm here ..."  '[skip 1]
[290F]     LOADSTR "hboc.hnm"
[291A]     SAY "We're in the middle of a conversation with king Betakam ..."
[2938]     SAY "You're here , Betakam ??? I can hardly believe it ... I spent years studying your civilization ..."  '[skip 1]
[2964]     LOADSTR "hboc.hnm"
[296F]     SAY "Commander how about teleporting his mummy, perhaps it would please him ..."
[298F]     SAY "What are you doing ? You're going to seriously traumatize the poor guy !"
[29B3]     SAY "Betakam... We know your sun Gladis is going to explode and all your descendants will perish ..."  '[skip 1]
[29DD]     LOADSTR "hboc.hnm"
[29E8]     SAY "How amusing ... You realize of course that nobody can predict the future !"
[2A0C]     SAY "Oh yes they can !!! Betakam , we know the future ... That's where we come from ..."  '[skip 1]
[2A38]     LOADSTR "hboc.hnm"
[2A43]     SAY "From the future , eh ? I suppose you forgot to bring proof ..."
[2A67]     SAY "Teleport him some technological gadget , Commander ..."
[2A7F]     SAY "I know , Commander... The Mind Scrambler !"
[2A97]     SAY "Excellent idea , Mister Honk !!!"  '[skip 1]
[2AAB]     LOADSTR "hboc.hnm"
[2AB6]     SAY "TELEPORT MIND SCRAMBLER TO KING BETAKAM IV word_65535 teleport refuse"
[2AD4]     IF-BLOCK (exit -> @2AF1)
[2AD7]       GUARD concept == "teleport"
[2ADA]       ENDIF
[2ADB]       SAY "TELEPORTING MIND SCRAMBLER"  '[skip 2]
[2AE9]       OP_CD CD 30 00 B6 14 22 01
[2AF0]       CLEAR concept_alt
    END
[2AF1]     IF-BLOCK (exit -> @2B13)
[2AF4]       GUARD concept == "refuse"
[2AF7]       ENDIF
[2AF8]       SAY "If that's the way you want it , Commander..."  '[skip 1]
[2B12]       CLEAR concept_alt
    END
[2B13]     IF-BLOCK (exit -> @2BF3)
[2B16]       GUARD rec_14CA == 290
[2B1B]       ENDIF
[2B1C]       SAY "What an intriguing device ... I'm beginning to believe you ..."
[2B3A]       SAY "Okay , Mister Betakam , what we can do is try to find a viable planet for you ..."
[2B68]       SAY "We're gonna teleport and transport you ..."  '[skip 1]
[2B7E]       LOADSTR "hboc.hnm"
[2B89]       SAY "All right ..."
[2B97]       SAY "Nice and easy , Commander ... This is all new to him ..."
[2BB9]       SAY "TELEPORT BETAKAM IV TO CRYOBOX word_65535 teleport"
[2BD1]       IF-BLOCK (exit -> @2BF3)
[2BD4]         GUARD concept == "teleport"
[2BD7]         ENDIF
[2BD8]         SAY "TELEPORTING BETAKAM IV TO ARK"  '[skip 3]
[2BEA]         rec_013A = 65535
[2BEF]         CLEAR concept_alt
[2BF0]         END PRESENTATION Betakam.talk
      END
    END
[2BF3]     SAY "I don't know about you , but I have some murffalo meat in the oven ..."
[2C1B]     SAY "..."  '[voice 1]
[2C25]     SAY "..."  '[voice 1]
[2C2F]     SAY "..."  '[voice 1]
[2C39]     SAY "See you soon ..."  '[skip 1]
[2C49]     END PRESENTATION Betakam.talk
  END
[2C4C]   BLOCK (exit -> @2D3D)
[2C50]     AWAIT gameflag_274F
[2C51]     GUARD active_actor == Betakam.talk (related 40)
[2C56]     ENDIF
[2C57]     SAY "Did you find a planet ?"
[2C6B]     IF-BLOCK (exit -> @2CEE)
[2C6E]       GUARD rec_1148 == 2840
[2C73]       ENDIF
[2C74]       SAY "We're on a fine planet ... An abandoned Croolis base ..."
[2C92]       SAY "All right ... leave me on Sat ... The murffalo intestines are favorable ..."
[2CB6]       SAY "TELEPORT BETAKAM TO SAT word_65535 teleport"
[2CCC]       IF-BLOCK (exit -> @2CEE)
[2CCF]         GUARD concept == "teleport"
[2CD2]         ENDIF
[2CD3]         SAY "TELEPORTING BETAKAM TO PLANET SAT"  '[skip 3]
[2CE5]         rec_013A = 2840
[2CEA]         CLEAR concept_alt
[2CEB]         END PRESENTATION Betakam.talk
      END
    END
[2CEE]     IF-BLOCK (exit -> @2D3D)
[2CF1]       GUARD NOT rec_1148 == 2840
[2CF7]       ENDIF
[2CF8]       SAY "That planet doesn't have good vibrations ..."
[2D0E]       SAY "The murffalo intestines are not favorable ..."
[2D24]       SAY "I don't wish to go there ..."  '[skip 1]
[2D3A]       END PRESENTATION Betakam.talk
    END
  END
[2D3D]   BLOCK (exit -> @2DCA)
[2D41]     AWAIT gameflag_252A
[2D42]     GUARD active_actor == Betakam.talk (related 40)
[2D47]     GUARD rec_1148 == 2840
[2D4C]     ENDIF
[2D4D]     SAY "It's not bad ... I'm enjoying it ..."
[2D65]     SAY "Makes a change ..."
[2D75]     SAY "It's more up to date , you know ? ... Stinks of oil ..."
[2D99]     SAY "Well , I have to go read the murffalo intestines ..."
[2DB7]     SAY "Bye , friends ..."  '[skip 1]
[2DC7]     END PRESENTATION Betakam.talk
  END
[2DCA]   BLOCK (exit -> @2E76)
[2DCE]     AWAIT gameflag_252A
[2DCF]     GUARD active_actor == Super_Tromp.talk (related 40)
[2DD4]     GUARD rec_1148 == 3164
[2DD9]     ENDIF
[2DDA]     SAY "Welcome to the planet Vista , stranger ..."  '[voice 0]
[2DF2]     SAY "How may I help you ?"  '[voice 1, skip 1]
[2E06]     rec_01F8 = 1
[2E0B]     SAY "You're leaving ? See you again , stranger ... word_65535 bye_bye"  '[voice 2, skip 1]
[2E29]     END PRESENTATION Super_Tromp.talk
[2E2C]     IF-BLOCK (exit -> @2E76)
[2E2F]       GUARD yo == 1
[2E36]       ENDIF
[2E37]       SAY "You wish to visit the tomb of the Great Yolk ?"  '[voice 2]
[2E55]       SAY "Enter with respect ... This is a sacred place ..."  '[voice 3, skip 1]
[2E71]       OP_C1 C1 F0 14 7A 0C
    END
  END
[2E76]   BLOCK (exit -> @3084)
[2E7A]     AWAIT gameflag_252A
[2E7B]     GUARD rec_1148 == 3164
[2E80]     GUARD active_actor == Sinox.talk (related 40)
[2E85]     ENDIF
[2E86]     SAY "Shhh ... Do not make noise , stranger ..."  '[voice 0]
[2EA0]     SAY "For ten centuries , the Great Yolk has rested in peace in this mausoleum ..."  '[voice 1]
[2EC6]     SAY "The mausoleum was built by Jeph d'Ulikan , who defeated the Black Larsen Hordes in the year 453765..."  '[voice 2]
[2EF2]     SAY "You may light a murffalo grease candle . It will bring you good fortune ..."  '[voice 3]
[2F18]     SAY "LIGHT A CANDLE ... word_65535 accept refuse"
[2F30]     IF-BLOCK (exit -> @2F50)
[2F33]       GUARD concept == "accept"
[2F36]       ENDIF
[2F37]       SAY "Your candle will burn for three years ..."  '[voice 4]
[2F4F]       CLEAR concept_alt
    END
[2F50]     IF-BLOCK (exit -> @2F6C)
[2F53]       GUARD concept == "refuse"
[2F56]       ENDIF
[2F57]       SAY "You're wasting a fine opportunity ..."  '[voice 5]
[2F6B]       CLEAR concept_alt
    END
[2F6C]     SAY "You cannot see the portrait of the Great Yolk , painted with his own yellow blood ..."  '[voice 6]
[2F96]     SAY "It is currently being restored not far from here ..."  '[voice 0]
[2FB2]     SAY "The painting is magical ... It gives its possessor Wisdom and Strength ..."  '[voice 7]
[2FD4]     SAY "Thousands of Tromps died to preserve the masterpiece ..."  '[voice 1]
[2FEE]     SAY "Legend says the painting's yellow blood becomes liquid every 100,000 years ..."  '[voice 2]
[300E]     SAY "That date has not yet come , unfortunately ... So we don't know if the story is true ..."  '[voice 2]
[303C]     SAY "The visit is now over ..."  '[voice 3]
[3050]     SAY "Goodbye , space pilgrim ... May the Great Yolk go with you ..."  '[voice 0]
[3072]     SAY "..."  '[voice 1, skip 2]
[307C]     rec_0212 = 4262
[3081]     END PRESENTATION Sinox.talk
  END
[3084]   BLOCK (exit -> @30C6)
[3088]     AWAIT gameflag_274F
[3089]     GUARD (rec_0C5E & 0x2) == 0
[308F]     GUARD active_actor == Anna_Haf.talk (related 40)
[3094]     ENDIF
[3095]     SAY "..."
[309F]     SAY "BROKEN ROBOT word_65535 bye_bye"
[30B1]     IF-BLOCK (exit -> @30C6)
[30B4]       GUARD concept == "bye_bye"
[30B7]       ENDIF
[30B8]       SAY "..."  '[skip 1]
[30C2]       END PRESENTATION Anna_Haf.talk
[30C5]       CLEAR concept_alt
    END
  END
[30C6]   BLOCK (exit -> @31EF)
[30CA]     AWAIT gameflag_274F
[30CB]     GUARD (rec_0C5E & 0x2) != 0
[30D0]     GUARD active_actor == Anna_Haf.talk (related 40)
[30D5]     ENDIF
[30D6]     SAY "I've got him fixed , Commander ... His name's Sighs Ate Anna Haf ..."
[30FA]     SAY "His multiplexors are shot to hell ..."
[3110]     SAY "Hello Mister Haf . You may now address our onboard comander ..."
[3130]     SAY "Hello , Commander... I am Robot Sighs Ate Anna Haf ..."  '[voice 0]
[314E]     SAY "I am descended from the noble Anna Haf family, naturally ..."  '[voice 1]
[316C]     SAY "Alas , I fell upon hard times ..."  '[voice 2]
[3184]     SAY "So I was forced to work ... Burglary , mostly ..."  '[voice 3]
[31A2]     SAY "If you need any burglaries done , I'm your robot !"  '[voice 4]
[31C0]     SAY "I'd call that somewhat rich possibility-wise . Right , Commander ?"
[31DE]     SAY "..."  '[skip 2]
[31E8]     POKE [0x30C7] = 0
[31EC]     END PRESENTATION Anna_Haf.talk
  END
[31EF]   BLOCK (exit -> @320C)
[31F3]     GUARD (rec_0C5E & 0x2) != 0
[31F8]     GUARD rec_0230 > 0
[31FF]     ENDIF
[3200]     POKE [0x3085] = 0
[3204]     POKE [0x320D] = 1
[3208]     POKE [0x31F0] = 0
  END
[320C]   GOTO @3362
[3210]   AWAIT gameflag_274F
[3211]   START PRESENTATION Anna_Haf.talk (related 40)
[3216]   ENDIF
[3217]   IF-BLOCK (exit -> @32DF)
[321A]     GUARD rec_1148 == 3164
[321F]     ENDIF
[3220]     SAY "Howzabout having Haf here snatch the portrait from Vista , Commander ?"
[3240]     SAY "Honk filled me in on the problem , Commander . An easy job ..."  '[voice 2]
[3264]     SAY "TELEPORT HAF TO PLANET VISTA word_65535 teleport refuse"
[327E]     IF-BLOCK (exit -> @32A4)
[3281]       GUARD concept == "teleport"
[3284]       ENDIF
[3285]       SAY "TELEPORTING HAF TO PLANET VISTA"  '[skip 4]
[3297]       rec_05BA = 3194
[329C]       POKE [0x320D] = 0
[32A0]       CLEAR concept_alt
[32A1]       END PRESENTATION Anna_Haf.talk
    END
[32A4]     IF-BLOCK (exit -> @32DF)
[32A7]       GUARD concept == "refuse"
[32AA]       ENDIF
[32AB]       SAY "Whatever you say , Commander ..."
[32BF]       SAY "Maybe you ought to think about it , Commander ..."  '[voice 2, skip 2]
[32DB]       CLEAR concept_alt
[32DC]       END PRESENTATION Anna_Haf.talk
    END
  END
[32DF]   SAY "He could help us get our hands on that painting , Commander ..."
[3301]   SAY "Quite true , Commander . Honk has brought me up to speed on the situation ..."  '[voice 2]
[3329]   SAY "Think about it , Commander... We just have to approach Vista and say the word to Haf ..."
[3355]   SAY "..."  '[skip 1]
[335F]   END PRESENTATION Anna_Haf.talk
[3362]   BLOCK (exit -> @33DC)
[3366]     AWAIT gameflag_252A
[3367]     GUARD rec_1148 == 3164
[336C]     GUARD active_actor == Anna_Haf.talk (related 40)
[3371]     ENDIF
[3372]     SAY "Don't stay here , Commander . I'll look for the painting ... There are some cases to examine ..."  '[voice 0]
[33A0]     SAY "See you later , Commander ..."  '[voice 1]
[33B4]     SAY "Bye bye ... Leave now and let me do my job ..."  '[voice 2, skip 2]
[33D4]     rec_05A4 |= 0x2
[33D9]     END PRESENTATION Anna_Haf.talk
  END
[33DC]   BLOCK (exit -> @3518)
[33E0]     AWAIT presentation
[33E1]     GUARD active_actor == Anna_Haf.talk (related 40)
[33E6]     ENDIF
[33E7]     IF-BLOCK (exit -> @3460)
[33EA]       GUARD NOT rec_1148 == 3164
[33F0]       ENDIF
[33F1]       SAY "KKKKK hello... KKKK RRR... KRUIK... KRUAK..."
[3405]       SAY "Sighs Ate Anna Haf here . This is a lousy line , Commander..."
[3427]       SAY "We're too far away . We'll need to get closer to the planet Vista .."
[344D]       SAY "KL KL KL KRUIKKKk..."  '[skip 1]
[345D]       END PRESENTATION Anna_Haf.talk
    END
[3460]     IF-BLOCK (exit -> @3518)
[3463]       GUARD rec_1148 == 3164
[3468]       ENDIF
[3469]       SAY "Is that you , Commander ? You're coming in loud and clear ..."
[348B]       SAY "I found the painting . It's protected by a sophisticated security system ..."
[34AD]       SAY "I can take care of it , Commander ... I'll get back to you when I've finished ..."
[34D9]       SAY "Until then, just let me work in peace ..."
[34F3]       SAY "KRUIKK..."
[34FD]       SAY "..."  '[skip 4]
[3507]       rec_05A4 &= !0x2
[350D]       state[7] = 60
[3511]       POKE [0x33DD] = 0
[3515]       END PRESENTATION Anna_Haf.talk
    END
  END
[3518]   BLOCK (exit -> @352C)
[351C]     GUARD state[7] == 0
[351E]     ENDIF
[351F]     OP_C3 C3 DC 05 28 00
[3524]     POKE [0x352D] = 1
[3528]     POKE [0x3519] = 0
  END
[352C]   GOTO @35BD
[3530]   AWAIT presentation
[3531]   rec_05BA = 3194
[3536]   START PRESENTATION Anna_Haf.talk (related 40)
[353B]   ENDIF
[353C]   SAY "Come in , Commander ... I'VE GOT THE PORTRAIT OF THE GREAT YOLK !"
[3560]   SAY "YES !!! Commander, he did it !"
[3576]   SAY "You better come get me ... I don't want to hang around here..."
[3598]   SAY "Hurry , Commander..."
[35A6]   SAY "Kruikkk..."  '[skip 3]
[35B0]   POKE [0x3363] = 0
[35B4]   rec_05A4 &= !0x2
[35BA]   END PRESENTATION Anna_Haf.talk
[35BD]   BLOCK (exit -> @3683)
[35C1]     AWAIT gameflag_252A
[35C2]     GUARD active_actor == Anna_Haf.talk (related 40)
[35C7]     ENDIF
[35C8]     SAY "Come in , Commander ... I'VE GOT THE PORTRAIT OF THE GREAT YOLK !"  '[voice 2]
[35EC]     SAY "YES !!! Commander, he did it !"
[3602]     SAY "TELEPORT PAINTING TO ARK word_65535 teleport"
[3618]     IF-BLOCK (exit -> @3637)
[361B]       GUARD concept == "teleport"
[361E]       ENDIF
[361F]       SAY "TELEPORTING PAINTING TO ARK"  '[skip 2]
[362F]       OP_CD CD DC 05 16 12 28 00
[3636]       CLEAR concept_alt
    END
[3637]     SAY "Teleport me , Commander . Hurry ..."  '[voice 4]
[364D]     SAY "Teleport him , Commander... word_65535 teleport"
[3663]     IF-BLOCK (exit -> @3683)
[3666]       GUARD concept == "teleport"
[3669]       ENDIF
[366A]       SAY "TELEPORT HAF TO ARK"  '[skip 3]
[367A]       rec_05BA = 65535
[367F]       CLEAR concept_alt
[3680]       END PRESENTATION Anna_Haf.talk
    END
  END
[3683]   BLOCK (exit -> @36CD)
[3687]     AWAIT gameflag_274F
[3688]     GUARD active_actor == Anna_Haf.talk (related 40)
[368D]     ENDIF
[368E]     SAY "Hello , Commander... I'm sleeping real well ..."  '[voice 2]
[36A6]     SAY "Just call if you need me ..."  '[voice 4]
[36BC]     SAY "Nite nite ..."  '[voice 5, skip 1]
[36CA]     END PRESENTATION Anna_Haf.talk
  END
[36CD]   BLOCK (exit -> @381D)
[36D1]     AWAIT gameflag_252A
[36D2]     GUARD active_actor == Rotator.talk (related 40)
[36D7]     ENDIF
[36D8]     SAY "Halt ! Who goes there ? I am ROTATOR , second class Croolis ventriloquist ..."  '[voice 1]
[36FE]     SAY "Careful , stranger . Don't try anything ... My laser-pulper could do bad things to your body ..."  '[voice 2]
[372A]     SAY "Rotator's laser-pulper is set to maximum pulp position ..."  '[voice 3]
[3744]     SAY "Commander , this could be tricky ... I'm scared ..."
[3760]     SAY "My chief will want to see you ... Your toes will be toasted and your eardrums punctured in several places ..."  '[voice 3]
[3792]     SAY "Your limbs will be reduced to paste texture . Your nose will be removed , your skin peeled off ..."  '[voice 4]
[37C2]     SAY "You will be required to eat your teeth ... repeatedly ..."  '[voice 5]
[37E0]     SAY "Co Co... Commander... Maybe this isn't such a good place to be ..."
[3802]     SAY "The CHIEF is waiting ..."  '[skip 2]
[3814]     OP_C1 C1 F0 14 A8 0B
[3819]     POKE [0x36CE] = 0
  END
[381D]   BLOCK (exit -> @3BA1)
[3821]     AWAIT gameflag_252A
[3822]     GUARD active_actor == Outrageor.talk (related 40)
[3827]     ENDIF
[3828]     SAY "Enter , Commander ... Surprised that I call you Commander ???"  '[voice 0]
[3846]     SAY "Our radar has been tracking you for some time ..."  '[voice 1]
[3862]     SAY "We know you visited the Tromps ... I've sent a full report to General EVISCERATOR..."  '[voice 2]
[3888]     SAY "This is serious , Commander . These folks are using this base to prepare for war !"
[38B2]     SAY "THEY 'RE BUILDING MORNING OILS !!!"
[38C6]     SAY "We're going to eliminate you ... WE KNOW YOU POSSESS THE CROOLIS WAR TREASURE ..."  '[voice 3]
[38EC]     SAY "What ? We don't have any such thing !"
[3906]     SAY "Excuse me , Commander ... But I'm having trouble getting to sleep ..."  '[skip 1]
[3928]     LOADSTR "hboc.hnm"
[3933]     SAY "This isn't a good moment , Mister Beauregard ..."
[394D]     SAY "I have something on my conscience... Something I have to confess to you ..."  '[skip 1]
[3971]     LOADSTR "hboc.hnm"
[397C]     SAY "The treasure ... Commander ... EVISCERATOR's treasure ... You see , I found it , Commander..."  '[skip 1]
[39A4]     LOADSTR "hboc.hnm"
[39AF]     SAY "It's ... IN THE MUMMY ..."  '[skip 1]
[39C3]     LOADSTR "hboc.hnm"
[39CE]     SAY "Oh boy That wasn't the smartest thing you ever did , Mister Beauregard..."
[39F0]     SAY "Well , I did intend telling you ..."  '[skip 1]
[3A08]     LOADSTR "hboc.hnm"
[3A13]     SAY "Looks like we'd better hand over the mummy , Commander ..."
[3A31]     SAY "If you've quite finished your nice little conversation ... I'd like your attention ..."  '[voice 5]
[3A55]     SAY "He's pretty persuasive , I find ..."
[3A6B]     SAY "No problem We'll teleport you the treasure ... It's safe inside a mummy ..."
[3A8F]     SAY "We know it's safe inside a mummy . We're the ones who hid it there !"  '[voice 6]
[3AB7]     SAY "TELEPORT MUMMY TO CROOLIS OUTRAGEOR word_65535 teleport"
[3ACF]     IF-BLOCK (exit -> @3AEE)
[3AD2]       GUARD concept == "teleport"
[3AD5]       ENDIF
[3AD6]       SAY "TELEPORTING MUMMY TO OUTRAGEOR"  '[skip 2]
[3AE6]       OP_CD CD 30 00 FE 11 6A 01
[3AED]       CLEAR concept_alt
    END
[3AEE]     SAY "Ha! Ha! Ha! Everything in its place , as I always say ..."  '[voice 5]
[3B10]     SAY "LET MISTER MAXXON GO FREE ..."
[3B24]     SAY "HA! HA! HA! I'll think about it ... maybe ... Ha! Ha! Ha!"  '[voice 4]
[3B46]     SAY "We're dealing with an exceptionally witty Croolis here , Commander ..."
[3B64]     SAY "Get out of here ... Don't let me see you again !"  '[voice 1]
[3B84]     SAY "Bye bye"  '[skip 4]
[3B90]     state[15] = 60
[3B94]     scrujo.talk = 4262
[3B99]     rec_037A = 2984
[3B9E]     END PRESENTATION Outrageor.talk
  END
[3BA1]   BLOCK (exit -> @3CE0)
[3BA5]     AWAIT gameflag_252A
[3BA6]     GUARD active_actor == Rotator.talk (related 40)
[3BAB]     ENDIF
[3BAC]     SAY "Halt ! Who goes there ? I am ROTATOR , second class Croolis ventriloquist ..."  '[voice 1]
[3BD2]     SAY "Careful , stranger . Don't try anything ... My laser-pulper could do bad things to your body ..."  '[voice 2]
[3BFE]     SAY "Rotator's laser-pulper is set to maximum pulp position ..."  '[voice 3]
[3C18]     SAY "Commander , this could be tricky ... I'm scared ..."
[3C34]     SAY "Your toes will be toasted and your eardrums punctured in several places ..."  '[voice 3]
[3C56]     SAY "Your limbs will be reduced to paste texture . Your nose will be removed , your skin peeled off ..."  '[voice 4]
[3C86]     SAY "You will be required to eat your teeth ... repeatedly ..."  '[voice 5]
[3CA4]     SAY "Co Co... Commander... Maybe this isn't such a good place to be ..."
[3CC6]     SAY "BAAAOOOM..."  '[voice 5, skip 2]
[3CD0]     LOADSTR "explo3.hnm"
[3CDD]     END PRESENTATION Rotator.talk
  END
[3CE0]   BLOCK (exit -> @3CF4)
[3CE4]     GUARD state[15] == 0
[3CE6]     ENDIF
[3CE7]     OP_C3 C3 9C 03 28 00
[3CEC]     POKE [0x3CF5] = 1
[3CF0]     POKE [0x3CE1] = 0
  END
[3CF4]   GOTO @3E17
[3CF8]   AWAIT presentation
[3CF9]   Eviscarator = dialog.value
[3D00]   ENDIF
[3D01]   SAY "Come in , Commander !!! This is General EVISCERATOR ..."  '[voice 0]
[3D1D]   SAY "I'm mad as mad can be , Commander ... Your mummy has laid a CURSE on us ..."
[3D49]   SAY "We're all sick... having nightmares ... We see mummies all over the place ... Even on TV ..."
[3D75]   SAY "You win ..."
[3D83]   SAY "We're letting MAXXON go ..."
[3D95]   SAY "He's a prisoner on a ship called the KUKARACHA . Coordinates x231 y654 ..."
[3DB9]   SAY "Commander !!! Whaddaya say !!! We're getting MAXXON back !!"
[3DD5]   SAY "In return , you take the CURSE off of us ..."
[3DF3]   SAY "OK ?... We're waiting ..."
[3E05]   SAY "KRUIIK"  '[skip 2]
[3E0F]   rec_1194 |= 0x2
[3E14]   END PRESENTATION Eviscerator.talk
[3E17]   BLOCK (exit -> @3E2E)
[3E1B]     GUARD (rec_1194 & 0x2) != 0
[3E20]     ENDIF
[3E21]     OP_C3 C3 04 05 28 00
[3E26]     POKE [0x3E2F] = 1
[3E2A]     POKE [0x3E18] = 0
  END
[3E2E]   GOTO @3EDF
[3E32]   AWAIT presentation
[3E33]   START PRESENTATION Jerry_Khan.talk (related 40)
[3E38]   ENDIF
[3E39]   SAY "Come in Commander ... This is Inspector Jerry Khan..."
[3E53]   SAY "Do you have any news ?"
[3E67]   SAY "Sure do , Inspector . We know where Maxxon is ... On a ship called Kukaracha ..."
[3E91]   SAY "Good work !!! Kukaracha ... I'll send Yoko to pick up his father ..."
[3EB5]   SAY "See you ..."
[3EC3]   SAY "KRUIIK"  '[skip 4]
[3ECD]   rec_040A = 4498
[3ED2]   rec_04CC &= !0x2
[3ED8]   POKE [0x3E2F] = 0
[3EDC]   END PRESENTATION Jerry_Khan.talk
[3EDF]   BLOCK (exit -> @3F94)
[3EE3]     AWAIT gameflag_252A
[3EE4]     GUARD active_actor == Yoko.talk (related 40)
[3EE9]     GUARD rec_1148 == 4498
[3EEE]     ENDIF
[3EEF]     SAY "Me happy see you , Commander ..."
[3F05]     SAY "Hurry , I want see father MAXXON ..."  '[skip 1]
[3F1D]     LOADSTR "aready30.hnm"
[3F2C]     SAY "Behind that door ..."  '[skip 1]
[3F3C]     LOADSTR "aready40.hnm"
[3F4B]     SAY "Teleport them , Commander, quick ... word_65535 teleport"
[3F65]     IF-BLOCK (exit -> @3F94)
[3F68]       GUARD concept == "teleport"
[3F6B]       ENDIF
[3F6C]       SAY "TELEPORTING MAXXON AND YOKO TO ARK"  '[skip 5]
[3F80]       rec_0722 = 65535
[3F85]       rec_040A = 65535
[3F8A]       rec_1194 &= !0x2
[3F90]       CLEAR concept_alt
[3F91]       END PRESENTATION Yoko.talk
    END
  END
[3F94]   BLOCK (exit -> @3FF6)
[3F98]     AWAIT gameflag_274F
[3F99]     GUARD active_actor == Yoko.talk (related 40)
[3F9E]     ENDIF
[3F9F]     SAY "Thank you... Thank you ... Commander..."
[3FB3]     SAY "Me happy see you , Commander ..."
[3FC9]     SAY "You have save my father..."
[3FDB]     SAY "Thank you..."
[3FE7]     SAY "Thank you..."  '[skip 1]
[3FF3]     END PRESENTATION Yoko.talk
  END
[3FF6]   BLOCK (exit -> @4046)
[3FFA]     AWAIT gameflag_274F
[3FFB]     GUARD active_actor == Maxxon.talk (related 40)
[4000]     ENDIF
[4001]     SAY "Thank you ... Commander..."
[4011]     SAY "It was terrible... Eviscerator is crazy..."
[4025]     SAY "The nightmare is off now..."
[4037]     SAY "Thank you..."  '[skip 1]
[4043]     END PRESENTATION Maxxon.talk
  END
[4046]   BLOCK (exit -> @406C)
[404A]     GUARD rec_013A == 2840
[404F]     GUARD rec_0722 == 65535
[4054]     GUARD rec_040A == 65535
[4059]     GUARD rec_0572 == 65535
[405E]     ENDIF
[405F]     OP_C3 C3 04 05 28 00
[4064]     POKE [0x406D] = 1
[4068]     POKE [0x4047] = 0
  END
[406C]   GOTO @4193
[4070]   AWAIT presentation
[4071]   START PRESENTATION Jerry_Khan.talk (related 40)
[4076]   ENDIF
[4077]   SAY "Come in, Commander ... This is Inspector JERRY KHAN on the SHARK..."
[4097]   SAY "I have good news ..."
[40A9]   SAY "I just captured Doctor Otto Von Smile ..."
[40C1]   SAY "I found the GLUXX kids ..."
[40D5]   SAY "We're going home , Commander..."
[40E7]   SAY "I've found the Oddland black hole..."
[40FB]   SAY "Oddland is at coordinates x465 Y342..."  '[skip 1]
[410F]   rec_1114 |= 0x2
[4114]   SAY "Say , Commander ... We've had quite an adventure ..."
[4130]   SAY "I'll see you on the other side of the black hole , Commander ..."
[4154]   SAY "This is Morning Oil : see you soon , Commander ..."
[4172]   SAY "BYE BYE COMMANDER ..."
[4182]   SAY "..."  '[skip 2]
[418C]   POKE [0x4194] = 1
[4190]   END PRESENTATION Jerry_Khan.talk
[4193]   GOTO @419F
[4197]   OP_C6 C6 4E 11 12 11
[419C]   ENDIF
[419D]   RUN PROFILE 4
[419F]   BLOCK (exit -> @41AC)
[41A3]     ENDIF
[41A4]     state[1] = 100
[41A8]     POKE [0x41A0] = 0
  END
[41AC]   BLOCK (exit -> @41C0)
[41B0]     GUARD state[1] == 0
[41B2]     ENDIF
[41B3]     OP_C3 C3 04 05 28 00
[41B8]     POKE [0x41C1] = 1
[41BC]     POKE [0x41AD] = 0
  END
[41C0]   GOTO @429F
[41C4]   AWAIT presentation
[41C5]   START PRESENTATION Jerry_Khan.talk (related 40)
[41CA]   ENDIF
[41CB]   SAY "Come in , Commander ... This is Inspector JERRY KHAN on the SHARK..."
[41ED]   SAY "We're through the black hole... An amazing experience , Commander ..."
[420B]   SAY "WE HAVE TRAVELLED SEVERAL HUNDRED THOUSAND YEARS BACK IN TIME ..."
[4229]   SAY "WE ARE COORDINATES X123 y432"
[423B]   SAY "COME AND JOIN US ..."
[424D]   SAY "Commander ... It's funny but I don't feel so good ..."
[426B]   SAY "GOOD LUCK ..."
[4279]   SAY "KRUIKKK..."
[4283]   SAY "..."  '[skip 4]
[428D]   rec_1170 |= 0x2
[4292]   rec_04CC &= !0x2
[4298]   POKE [0x41C1] = 0
[429C]   END PRESENTATION Jerry_Khan.talk
[429F]   BLOCK (exit -> @43DD)
[42A3]     AWAIT gameflag_252A
[42A4]     GUARD rec_1148 == 4462
[42A9]     GUARD active_actor == Jerry_Khan.talk (related 40)
[42AE]     ENDIF
[42AF]     SAY "Welcome aboard the Shark, Commander..."  '[voice 4]
[42C1]     SAY "What a crazy adventure ... have you visited the sector ?"  '[voice 5]
[42DF]     SAY "I identified the planet Ron . It's RONDO in fact ... Yoko's planet ..."  '[voice 6]
[4303]     SAY "Wow ... That's weird ..."
[4315]     SAY "I identified the planet Vistar ..."  '[voice 2]
[4329]     SAY "Vistar is at coordinates x564 y987..."  '[voice 3, skip 1]
[433D]     rec_0C28 |= 0x2
[4342]     SAY "Thank you Inspector ..."
[4352]     SAY "I'm going to start my investigation , Commander . We'll stay in radio contact ..."  '[voice 5]
[4378]     SAY "Eviscerator's hiding somewhere among those stars ."  '[voice 3]
[438E]     SAY "He mustn't be allowed to make trouble ..."  '[voice 4]
[43A6]     SAY "Well ... I wish you good luck , Commander ..."  '[voice 7]
[43C2]     SAY "..."  '[skip 4]
[43CC]     rec_1170 &= !0x2
[43D2]     POKE [0x42A0] = 0
[43D6]     POKE [0x43DE] = 1
[43DA]     END PRESENTATION Jerry_Khan.talk
  END
[43DD]   GOTO @43EA
[43E1]   ENDIF
[43E2]   state[13] = 200
[43E6]   POKE [0x43DE] = 0
[43EA]   BLOCK (exit -> @4400)
[43EE]     ENDIF
[43EF]     IF-BLOCK (exit -> @4400)
[43F2]       GUARD state[13] == 0
[43F4]       ENDIF
[43F5]       ta += 1
[43FC]       state[13] = 200
    END
  END
[4400]   BLOCK (exit -> @4411)
[4404]     GUARD ta == 1
[440B]     ENDIF
[440C]     OP_C3 C3 04 05 28 00
  END
[4411]   BLOCK (exit -> @44BA)
[4415]     AWAIT presentation
[4416]     GUARD active_actor == Jerry_Khan.talk (related 40)
[441B]     GUARD ta == 1
[4422]     ENDIF
[4423]     SAY "This is Inspector Jerry Khan..."  '[voice 2]
[4435]     SAY "Commander , I picked up a message for you from someone called CBRION JANIOR."
[4459]     SAY "The message was pretty garbled ... Probably because of the black hole ..."
[447B]     SAY "My investigation's going fine . I know where doctor Otto Von Smile is..."
[449D]     SAY "SEE YOU"
[44A9]     SAY "Kruikk..."  '[skip 2]
[44B3]     POKE [0x4401] = 0
[44B7]     END PRESENTATION Jerry_Khan.talk
  END
[44BA]   BLOCK (exit -> @44CB)
[44BE]     GUARD ta == 2
[44C5]     ENDIF
[44C6]     OP_C3 C3 04 05 28 00
  END
[44CB]   BLOCK (exit -> @454E)
[44CF]     AWAIT presentation
[44D0]     GUARD active_actor == Jerry_Khan.talk (related 40)
[44D5]     GUARD ta == 2
[44DC]     ENDIF
[44DD]     SAY "This is Jerry Khan..."  '[voice 2]
[44ED]     SAY "Everything okay , Commander ?"
[44FF]     SAY "Sure !!! We're doing great ..."
[4513]     SAY "Glad to hear it ... I'll call you if I need any help ..."
[4537]     SAY "See you , Commander..."  '[skip 2]
[4547]     POKE [0x44BB] = 0
[454B]     END PRESENTATION Jerry_Khan.talk
  END
[454E]   BLOCK (exit -> @4601)
[4552]     AWAIT presentation
[4553]     GUARD active_actor == menu.talk (related 40)
[4558]     GUARD rec_052A == 65535
[455D]     ENDIF
[455E]     SAY ""IMPROVED MENU""
[456A]     SAY "Today CHEF BRONKO has laid out for you :"
[4584]     SAY "Tasty MUFFALO soup Bronko-style ."
[4596]     SAY "MURFFALO kidneys Bronko-style ."
[45A6]     SAY "MURFFALO hamburger with Bar-B-Q recycled-oil dip ."
[45BC]     SAY "Smooth MURFFALO-chip ice cream ."
[45CE]     SAY "Recycled water"
[45DA]     SAY "Chef Bronko says ... Burping's bad manners ! ..."
[45F4]     SAY "stop"  '[skip 1]
[45FE]     END PRESENTATION menu.talk
  END
[4601]   BLOCK (exit -> @46B1)
[4605]     AWAIT presentation
[4606]     GUARD active_actor == menu.talk (related 40)
[460B]     GUARD NOT rec_052A == 65535
[4611]     ENDIF
[4612]     SAY ""MENU""
[461C]     SAY "Today's fare :"
[462A]     SAY "PLASMA soup HONK-style ."
[463A]     SAY "WRIGGLER belly in slobber sauce ."
[464E]     SAY "Jellied URTIKAN with MURFFALO bone marrow ."
[4664]     SAY "GLOK eye pie ."
[4674]     SAY "Recycled water"
[4680]     SAY "The chef says ... Don't eat with your mouth full ! ..."
[46A0]     SAY "Stop"  '[skip 2]
[46AA]     POKE [0x4602] = 0
[46AE]     END PRESENTATION menu.talk
  END
[46B1]   BLOCK (exit -> @475B)
[46B5]     AWAIT presentation
[46B6]     GUARD active_actor == menu.talk (related 40)
[46BB]     GUARD NOT rec_052A == 65535
[46C1]     ENDIF
[46C2]     SAY ""MENU""
[46CC]     SAY "Today's fare :"
[46DA]     SAY "PLASMA soup HONK-style ."
[46EA]     SAY "WRIGGLER snout stew ."
[46FA]     SAY "URTIKAN seeds in MURFFALO venom ."
[470E]     SAY "GLOK juice dessert ."
[471E]     SAY "Recycled water"
[472A]     SAY "The chef says ... Don't talk with your mouth open ! ..."
[474A]     SAY "stop"  '[skip 2]
[4754]     POKE [0x46B2] = 0
[4758]     END PRESENTATION menu.talk
  END
[475B]   BLOCK (exit -> @4807)
[475F]     AWAIT presentation
[4760]     GUARD NOT rec_052A == 65535
[4766]     GUARD active_actor == menu.talk (related 40)
[476B]     ENDIF
[476C]     SAY ""MENU""
[4776]     SAY "Today's fare :"
[4784]     SAY "PLASMA soup HONK-style ."
[4794]     SAY "WRIGGLER feet in emulsive sauce ."
[47A8]     SAY "URTIKAN leaves in MURFFALO sweat ."
[47BC]     SAY "GLOK flake dessert ."
[47CC]     SAY "Recycled water"
[47D8]     SAY "The chef says ... Somebody didn't finish his wrigglers yesterday ..."
[47F6]     SAY "stop"  '[skip 2]
[4800]     POKE [0x475C] = 0
[4804]     END PRESENTATION menu.talk
  END
[4807]   BLOCK (exit -> @48BB)
[480B]     AWAIT presentation
[480C]     GUARD active_actor == menu.talk (related 40)
[4811]     GUARD NOT rec_052A == 65535
[4817]     ENDIF
[4818]     SAY ""MENU""
[4822]     SAY "Today's fare :"
[4830]     SAY "HONK-style PLASMA soup ."
[4840]     SAY "WRIGGLER brain , stewed in its own juice ."
[485A]     SAY "URTIKAN trunk , stuffed with MURFFALO liver ."
[4872]     SAY "GLOK dee-lite ."
[4880]     SAY "Recycled water"
[488C]     SAY "The chef says ... Plenty more in the kitchen ! ..."
[48AA]     SAY "stop"  '[skip 2]
[48B4]     POKE [0x4808] = 0
[48B8]     END PRESENTATION menu.talk
  END
[48BB]   BLOCK (exit -> @4977)
[48BF]     AWAIT presentation
[48C0]     GUARD active_actor == menu.talk (related 40)
[48C5]     GUARD NOT rec_052A == 65535
[48CB]     ENDIF
[48CC]     SAY ""IMPROVED MENU""
[48D8]     SAY "Today's fare :"
[48E6]     SAY "Soup of PLASMA HONK-style ."
[48F8]     SAY "WRIGGLER hearts in green blood coagulate ."
[490E]     SAY "URTIKAN roots , deep fried in recycled oil ."
[4928]     SAY "Candied GLOK tongue ."
[4938]     SAY "Recycled water"
[4944]     SAY "The chef says ... You eat what you are ! ..."
[4962]     SAY "stop"  '[skip 3]
[496C]     POKE [0x48BC] = 0
[4970]     POKE [0x4978] = 1
[4974]     END PRESENTATION menu.talk
  END
[4977]   GOTO @4994
[497B]   ENDIF
[497C]   POKE [0x4602] = 1
[4980]   POKE [0x46B2] = 1
[4984]   POKE [0x475C] = 1
[4988]   POKE [0x4808] = 1
[498C]   POKE [0x48BC] = 1
[4990]   POKE [0x4978] = 0
[4994]   BLOCK (exit -> @4BB8)
[4998]     AWAIT presentation
[4999]     GUARD active_actor == Honk.talk (related 40)
[499E]     ENDIF
[499F]     SAY "I exist only to obey , Commander"
[49B5]     IF-BLOCK (exit -> @4A2A)
[49B8]       GUARD vbio == 0
[49BF]       ENDIF
[49C0]       SAY "Commander , we don't have any BIONIUM ... COMMANDER , please ..."
[49E0]       SAY "I need that energy ..."
[49F2]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[4A08]       SAY "Wake up Scruter Jo , Commander . He's sleeping in the Cryobox ..."
    END
[4A2A]     IF-BLOCK (exit -> @4A8B)
[4A2D]       GUARD vbio == 1
[4A34]       ENDIF
[4A35]       SAY "We've got one dose of BIONIUM left , Commander"
[4A4F]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[4A65]       SAY "I don't feel too sure of myself , Commander... I really need that energy ..."
    END
[4A8B]     IF-BLOCK (exit -> @4AC6)
[4A8E]       GUARD vbio == 2
[4A95]       ENDIF
[4A96]       SAY "We've got two doses of BIONIUM left , Commander"
[4AB0]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
    END
[4AC6]     IF-BLOCK (exit -> @4AEB)
[4AC9]       GUARD vbio == 3
[4AD0]       ENDIF
[4AD1]       SAY "We've got three doses of BIONIUM left , Commander"
    END
[4AEB]     IF-BLOCK (exit -> @4B10)
[4AEE]       GUARD vbio == 4
[4AF5]       ENDIF
[4AF6]       SAY "We've got four doses of BIONIUM left , Commander"
    END
[4B10]     IF-BLOCK (exit -> @4B35)
[4B13]       GUARD vbio == 5
[4B1A]       ENDIF
[4B1B]       SAY "We've got five doses of BIONIUM left , Commander"
    END
[4B35]     IF-BLOCK (exit -> @4B5A)
[4B38]       GUARD vbio == 6
[4B3F]       ENDIF
[4B40]       SAY "We've got six doses of BIONIUM left , Commander"
    END
[4B5A]     IF-BLOCK (exit -> @4B7F)
[4B5D]       GUARD vbio == 7
[4B64]       ENDIF
[4B65]       SAY "We've got seven doses of BIONIUM left , Commander"
    END
[4B7F]     IF-BLOCK (exit -> @4BB8)
[4B82]       GUARD vbio == 8
[4B89]       ENDIF
[4B8A]       SAY "We've got eight doses of BIONIUM left , Commander"
[4BA4]       SAY "You're the best , Commander ..."
    END
  END
[4BB8]   BLOCK (exit -> @4C34)
[4BBC]     AWAIT presentation
[4BBD]     GUARD active_actor == Honk.talk (related 40)
[4BC2]     GUARD vbio > 2
[4BC9]     GUARD rec_05BA == 65535
[4BCE]     GUARD NOT rec_122A == 65535
[4BD4]     GUARD NOT rec_122A == 2450
[4BDA]     GUARD (rec_0C5E & 0x2) != 0
[4BDF]     ENDIF
[4BE0]     SAY "Go near Vista , and talk to Anna Haf in the cryobox ..."  '[skip 1]
[4C02]     vbio -= 3
[4C09]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[4C2D]     POKE [0x4BB9] = 0
[4C31]     END PRESENTATION Honk.talk
  END
[4C34]   BLOCK (exit -> @4C95)
[4C38]     AWAIT presentation
[4C39]     GUARD active_actor == Honk.talk (related 40)
[4C3E]     GUARD vbio > 2
[4C45]     GUARD rec_14E2 == 65535
[4C4A]     ENDIF
[4C4B]     SAY "I'm sure Super Zen can help us ..."  '[skip 1]
[4C63]     vbio -= 3
[4C6A]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[4C8E]     POKE [0x4C35] = 0
[4C92]     END PRESENTATION Honk.talk
  END
[4C95]   BLOCK (exit -> @4CF8)
[4C99]     AWAIT presentation
[4C9A]     GUARD active_actor == Honk.talk (related 40)
[4C9F]     GUARD vbio > 2
[4CA6]     GUARD (rec_0B1A & 0x2) == 0
[4CAC]     GUARD rec_013A == 65535
[4CB1]     ENDIF
[4CB2]     SAY "Maziok can help us, Commander ..."  '[skip 1]
[4CC6]     vbio -= 3
[4CCD]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[4CF1]     POKE [0x4C96] = 0
[4CF5]     END PRESENTATION Honk.talk
  END
[4CF8]   BLOCK (exit -> @4D5E)
[4CFC]     AWAIT presentation
[4CFD]     GUARD active_actor == Honk.talk (related 40)
[4D02]     GUARD vbio > 2
[4D09]     GUARD rec_07D0 > 0
[4D10]     GUARD rec_1242 == 65535
[4D15]     ENDIF
[4D16]     SAY "Fifi 'll love this ondoyant picture ..."  '[skip 1]
[4D2C]     vbio -= 3
[4D33]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[4D57]     POKE [0x4CF9] = 0
[4D5B]     END PRESENTATION Honk.talk
  END
[4D5E]   BLOCK (exit -> @4DBF)
[4D62]     AWAIT presentation
[4D63]     GUARD active_actor == Honk.talk (related 40)
[4D68]     GUARD vbio > 2
[4D6F]     GUARD rec_07B2 == 65535
[4D74]     ENDIF
[4D75]     SAY "Fifi wants to go to planet Malus ..."  '[skip 1]
[4D8D]     vbio -= 3
[4D94]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[4DB8]     POKE [0x4D5F] = 0
[4DBC]     END PRESENTATION Honk.talk
  END
[4DBF]   BLOCK (exit -> @4DDF)
[4DC3]     AWAIT presentation
[4DC4]     GUARD active_actor == Honk.talk (related 40)
[4DC9]     ENDIF
[4DCA]     SAY "Bye bye , Commander ..."  '[skip 1]
[4DDC]     END PRESENTATION Honk.talk
  END
[4DDF]   BLOCK (exit -> @4E90)
[4DE3]     AWAIT gameflag_274F
[4DE4]     GUARD active_actor == Bob_Morlock.talk (related 40)
[4DE9]     ENDIF
[4DEA]     SAY "What do you want , Commander?"  '[voice 2]
[4DFE]     rec_0288 = 1
[4E03]     SAY "..."  '[skip 1]
[4E0D]     state[1] = 30
[4E11]     IF-BLOCK (exit -> @4E48)
[4E14]       GUARD state[1] == 0
[4E16]       ENDIF
[4E17]       SAY "I feel awful , Commander ..."  '[voice 6]
[4E2B]       SAY "Ahhhh !!!"  '[voice 3]
[4E37]       SAY "stop"  '[skip 2]
[4E41]       state[1] = 65535
[4E45]       END PRESENTATION Bob_Morlock.talk
    END
[4E48]     SAY "I feel weak ... word_65535 bye_bye"  '[voice 6, skip 1]
[4E5C]     adieu = 1
[4E63]     IF-BLOCK (exit -> @4E90)
[4E66]       GUARD adieu == 1
[4E6D]       ENDIF
[4E6E]       SAY "..."
[4E78]       SAY "stop"  '[skip 3]
[4E82]       adieu = 0
[4E89]       state[1] = 65535
[4E8D]       END PRESENTATION Bob_Morlock.talk
    END
  END
[4E90]   BLOCK (exit -> @4F90)
[4E94]     AWAIT gameflag_274F
[4E95]     GUARD active_actor == Bob_Morlock.talk (related 40)
[4E9A]     GUARD emplo == 1
[4EA1]     GUARD emp == 0
[4EA8]     ENDIF
[4EA9]     SAY "By the way , Cap'n Bob sir , I have a request to make ..."
[4ECF]     SAY "Olga and me want a raise ... Just a few more megawatts ..."
[4EF1]     SAY "WHAT ... YOU'LL HAVE TO SPEAK LOUDER ... I'M GETTING A BIT DEAF COMMANDER ..."  '[voice 5]
[4F17]     SAY "WE WANT A RAISE IN MEGAWATTS !!!"
[4F2D]     SAY "I'm feeling a bit sick , Commander . Aaah !!! It's my heart ...."  '[voice 4]
[4F51]     SAY "Heart my foot ... he means his power generator . They're all the same ... You just have to say the word raise ."  '[skip 1]
[4F89]     emp = 1
  END
[4F90]   BLOCK (exit -> @5157)
[4F94]     AWAIT gameflag_274F
[4F95]     GUARD active_actor == Bob_Morlock.talk (related 40)
[4F9A]     GUARD reve == 1
[4FA1]     GUARD revelat == 0
[4FA8]     ENDIF
[4FA9]     SAY "I'm going to tell you an unbearable truth , Commander :"  '[voice 7]
[4FC7]     SAY "HONK! Switch yourself off for ten seconds !!!"  '[voice 6]
[4FDF]     SAY "But Cap'n Bob ... I ..."
[4FF3]     SAY "I SAID SWITCH OFF , GOSH DARN IT !!!"  '[voice 5]
[500D]     SAY "Yes sir ....."
[501B]     SAY "KRUIIIIK !!! AAAaaaaaaaaaaaaaaaaaaaaa !!!"
[502B]     SAY "COMMANDER, YOU ARE ME ...."  '[voice 5]
[503D]     SAY "WE ARE THE SAME BEING AT TWO DIFFERENT AGES ..."  '[voice 6]
[5059]     SAY "YOU ARE MUCH MORE THAN A SON TO ME ..."  '[voice 4]
[5075]     SAY "We're the same person ... I am the first self-creating being !"  '[voice 5]
[5095]     SAY "Thanks to space-time contortion , I can relive my youth : YOU ARE BOB , COMMANDER ..."  '[voice 4]
[50BF]     SAY "I am what you'll be in several hundred thousand years ..."  '[voice 2]
[50DD]     SAY "OK Honk , you can switch on ..."  '[voice 6]
[50F5]     SAY "KROIIIIkkk !!! -&KRUIIIIkkk !!! -&Look , I'm supposed to know everything that goes on around here ! ... Don't tell me you switched off Olga ???"
[5131]     SAY "Cut the whining and get to work ..."  '[voice 5, skip 2]
[5149]     revelat = 1
[5150]     reve = 0
  END
[5157] END OF SCRIPT
