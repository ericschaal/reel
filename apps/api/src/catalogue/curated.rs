//! Curated category cards mirrored from Seerr's discovery interface.
//!
//! Seerr does not expose these lists as an endpoint. Keep the provider-specific
//! identifiers, display order, and artwork together so drift is obvious and the
//! catalogue orchestration remains about Reel product behaviour.

pub(super) struct CuratedCategory {
    pub id: i64,
    pub title: &'static str,
    pub image: &'static str,
}

pub(super) const STUDIOS: &[CuratedCategory] = &[
    CuratedCategory {
        id: 2,
        title: "Disney",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/wdrCwmRnLFJhEoH8GSfymY85KHT.png",
    },
    CuratedCategory {
        id: 127_928,
        title: "20th Century Studios",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/h0rjX5vjW5r8yEnUBStFarjcLT4.png",
    },
    CuratedCategory {
        id: 34,
        title: "Sony Pictures",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/GagSvqWlyPdkFHMfQ3pNq6ix9P.png",
    },
    CuratedCategory {
        id: 174,
        title: "Warner Bros. Pictures",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/ky0xOc5OrhzkZ1N6KyUxacfQsCk.png",
    },
    CuratedCategory {
        id: 33,
        title: "Universal",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/8lvHyhjr8oUKOOy2dKXoALWKdp0.png",
    },
    CuratedCategory {
        id: 4,
        title: "Paramount",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/fycMZt242LVjagMByZOLUGbCvv3.png",
    },
    CuratedCategory {
        id: 3,
        title: "Pixar",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/1TjvGVDMYsj6JBxOAkUHpPEwLf7.png",
    },
    CuratedCategory {
        id: 521,
        title: "Dreamworks",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/kP7t6RwGz2AvvTkvnI1uteEwHet.png",
    },
    CuratedCategory {
        id: 420,
        title: "Marvel Studios",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/hUzeosd33nzE5MCNsZxCGEKTXaQ.png",
    },
    CuratedCategory {
        id: 9993,
        title: "DC",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/2Tc1P3Ac8M479naPp1kYT3izLS5.png",
    },
    CuratedCategory {
        id: 41077,
        title: "A24",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/1ZXsGaFPgrgS6ZZGS37AqD5uU12.png",
    },
];

pub(super) const NETWORKS: &[CuratedCategory] = &[
    CuratedCategory {
        id: 213,
        title: "Netflix",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/wwemzKWzjKYJFfCeiB57q3r4Bcm.png",
    },
    CuratedCategory {
        id: 2739,
        title: "Disney+",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/gJ8VX6JSu3ciXHuC2dDGAo2lvwM.png",
    },
    CuratedCategory {
        id: 1024,
        title: "Prime Video",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/ifhbNuuVnlwYy5oXA5VIb2YR8AZ.png",
    },
    CuratedCategory {
        id: 2552,
        title: "Apple TV+",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/4KAy34EHvRM25Ih8wb82AuGU7zJ.png",
    },
    CuratedCategory {
        id: 453,
        title: "Hulu",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/pqUTCleNUiTLAVlelGxUgWn1ELh.png",
    },
    CuratedCategory {
        id: 49,
        title: "HBO",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/tuomPhY2UtuPTqqFnKMVHvSb724.png",
    },
    CuratedCategory {
        id: 4353,
        title: "Discovery+",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/1D1bS3Dyw4ScYnFWTlBOvJXC3nb.png",
    },
    CuratedCategory {
        id: 2,
        title: "ABC",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/ndAvF4JLsliGreX87jAc9GdjmJY.png",
    },
    CuratedCategory {
        id: 19,
        title: "FOX",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/1DSpHrWyOORkL9N2QHX7Adt31mQ.png",
    },
    CuratedCategory {
        id: 359,
        title: "Cinemax",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/6mSHSquNpfLgDdv6VnOOvC5Uz2h.png",
    },
    CuratedCategory {
        id: 174,
        title: "AMC",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/pmvRmATOCaDykE6JrVoeYxlFHw3.png",
    },
    CuratedCategory {
        id: 67,
        title: "Showtime",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/Allse9kbjiP6ExaQrnSpIhkurEi.png",
    },
    CuratedCategory {
        id: 318,
        title: "Starz",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/8GJjw3HHsAJYwIWKIPBPfqMxlEa.png",
    },
    CuratedCategory {
        id: 71,
        title: "The CW",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/ge9hzeaU7nMtQ4PjkFlc68dGAJ9.png",
    },
    CuratedCategory {
        id: 6,
        title: "NBC",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/o3OedEP0f9mfZr33jz2BfXOUK5.png",
    },
    CuratedCategory {
        id: 16,
        title: "CBS",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/nm8d7P7MJNiBLdgIzUK0gkuEA4r.png",
    },
    CuratedCategory {
        id: 4330,
        title: "Paramount+",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/fi83B1oztoS47xxcemFdPMhIzK.png",
    },
    CuratedCategory {
        id: 4,
        title: "BBC One",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/mVn7xESaTNmjBUyUtGNvDQd3CT1.png",
    },
    CuratedCategory {
        id: 56,
        title: "Cartoon Network",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/c5OC6oVCg6QP4eqzW6XIq17CQjI.png",
    },
    CuratedCategory {
        id: 80,
        title: "Adult Swim",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/9AKyspxVzywuaMuZ1Bvilu8sXly.png",
    },
    CuratedCategory {
        id: 13,
        title: "Nickelodeon",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/ikZXxg6GnwpzqiZbRPhJGaZapqB.png",
    },
    CuratedCategory {
        id: 3353,
        title: "Peacock",
        image: "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/gIAcGTjKKr0KOHL5s4O36roJ8p7.png",
    },
];
