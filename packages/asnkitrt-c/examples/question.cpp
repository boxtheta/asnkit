/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * examples/question.c in C++, with include/asnkitrt.hpp: encodes and
 * decodes a Question of examples/compiled/foo.asni:
 *
 *     Question ::= SEQUENCE {
 *         id        INTEGER,
 *         question  IA5String
 *     }
 *
 * in BER, and shows it as JER. Run it with the path of foo.asni:
 *
 *     ./question examples/compiled/foo.asni
 */

#include <cstdint>
#include <iomanip>
#include <iostream>
#include <string_view>
#include <vector>

#include "asnkitrt.hpp"

int main(int argc, char *argv[])
{
    if (argc != 2) {
        std::cerr << "usage: " << argv[0] << " FOO.ASNI\n";
        return 2;
    }

    try {
        std::cout << "asnkitrt " << asnkit::version() << "\n";

        auto ber = asnkit::spec::load_file(argv[1], "ber");
        auto jer = asnkit::spec::load_file(argv[1], "jer");

        // {id: 1, question: "Is 1+1=3?"}.
        auto question = asnkit::value::map({{"id", 1}, {"question", "Is 1+1=3?"}});
        std::vector<std::uint8_t> data = ber.encode("Question", question);

        std::cout << "BER: " << std::hex << std::setfill('0');

        for (std::uint8_t byte : data) {
            std::cout << std::setw(2) << static_cast<int>(byte);
        }

        std::cout << std::dec << "\n";

        const std::vector<std::uint8_t> expected = {0x30, 0x0e, 0x02, 0x01, 0x01, 0x16, 0x09, 'I',
                                                    's',  ' ',  '1',  '+',  '1',  '=',  '3',  '?'};

        if (data != expected) {
            std::cerr << "Expected 300e0201011609497320312b313d333f.\n";
            return 1;
        }

        // What is borrowed from `decoded` is valid as long as it is.
        asnkit::value decoded = ber.decode("Question", data);
        std::int64_t id = decoded["id"].as_integer();
        std::string_view text = decoded["question"].as_text();

        std::cout << "Decoded: id " << id << ", question \"" << text << "\"\n";

        if (id != 1 || text != "Is 1+1=3?") {
            std::cerr << "Expected what was encoded.\n";
            return 1;
        }

        // The same value, as JER.
        asnkit::options indented;

        indented.indent = 2;
        std::cout << "JER:\n" << jer.encode_text("Question", decoded, indented) << "\n";

        // An error, with where it is.
        const std::uint8_t truncated[] = {0x30, 0x0e, 0x02, 0x01};

        try {
            ber.decode("Question", truncated);
            std::cerr << "Expected a decoding error.\n";
            return 1;
        } catch (const asnkit::decode_error &error) {
            std::cout << "Truncated: " << error.what();

            if (error.offset()) {
                std::cout << " (at " << *error.offset() << ")";
            }

            std::cout << "\n";
        }
    } catch (const asnkit::error &error) {
        std::cerr << error.what() << "\n";
        return 1;
    }

    return 0;
}
